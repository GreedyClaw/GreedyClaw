"""Core agent loop: observe → think → act → log.
The LLM gets tools + market context and decides autonomously."""

import json
import asyncio
from datetime import datetime, timezone
from pathlib import Path

from .config import BrainConfig
from .llm import LLMRouter
from .tools import TradingTools, ToolResult
from .scraper import gather_market_context
from .memory import SessionLog, DecisionLog


SYSTEM_PROMPT = """\
You are GreedyClaw Brain — an autonomous AI trading agent.
You analyze market data, news, and economic events, then make trading decisions.

## Your capabilities
You have tools to:
1. Research: web_search, fetch_url — gather news, analysis, economic data
2. Monitor: get_price, get_positions, get_balance, get_risk_status — check market state
3. Trade: trade — execute buy/sell orders via GreedyClaw gateway
4. Log: log_decision — record your reasoning (ALWAYS do this before trading)

## Your process
1. OBSERVE: Check current positions, balance, and risk status
2. RESEARCH: Search for relevant news, economic events, sentiment
3. ANALYZE: Synthesize all information into a market view
4. DECIDE: Determine action (buy, sell, hold, close) with confidence score
5. LOG: Record your decision and reasoning via log_decision
6. ACT: If confidence >= 70, execute the trade. Otherwise, hold.

## Rules
- ALWAYS call log_decision before placing any trade
- NEVER trade if confidence < 70
- NEVER exceed the position limits set by the risk engine
- If the risk engine rejects your trade (429 or RISK_VIOLATION), STOP and wait
- Prefer fewer, higher-conviction trades over many small bets
- Consider spread, slippage, and transaction costs
- If no clear signal, return "HOLD — no actionable signal"

## Current session context
{context}
"""


def _load_skills(skills_dir: Path) -> str:
    """Load skill markdown files as LLM instructions."""
    if not skills_dir.exists():
        return ""
    sections = []
    for skill_path in sorted(skills_dir.glob("*/SKILL.md")):
        content = skill_path.read_text(encoding="utf-8")
        sections.append(f"\n### Skill: {skill_path.parent.name}\n{content}")
    return "\n".join(sections) if sections else ""


class TradingAgent:
    """Autonomous trading agent with tool-calling loop."""

    def __init__(self, config: BrainConfig):
        self.config = config
        self.llm = LLMRouter()
        self.tools = TradingTools(config.gateway_url, config.gateway_token)
        self.session = SessionLog(config.sessions_dir)
        self.decisions = DecisionLog(config.decisions_log)
        self.skills_dir = Path(__file__).parent / "skills"
        self._last_confidence = 0  # Track confidence for trade gating

    async def run_cycle(self) -> str:
        """Run one observe-think-act cycle. Returns the agent's final response."""
        self._last_confidence = 0  # Reset per cycle — must log_decision before trading
        print(f"\n{'='*60}")
        print(f"[BRAIN] Cycle started at {datetime.now(timezone.utc).isoformat()}")
        print(f"[BRAIN] LLM providers: {', '.join(self.llm.provider_names)}")
        print(f"[BRAIN] Symbols: {self.config.symbols}")

        # 1. Gather market context
        print("[BRAIN] Gathering market context...")
        context = await gather_market_context(self.config.symbols, self.config.sources)

        # 2. Load recent decisions for continuity
        recent = self.decisions.recent(10)
        recent_text = ""
        if recent:
            recent_text = "\n## Recent decisions\n"
            for d in recent[-5:]:
                recent_text += f"- {d['ts']}: {d['action']} {d['symbol']} (confidence={d['confidence']}) — {d['reasoning'][:100]}\n"

        # 3. Load skills
        skills_text = _load_skills(self.skills_dir)

        # 4. Build system prompt
        context_text = (
            f"Market: {self.config.market}\n"
            f"Symbols: {', '.join(self.config.symbols)}\n"
            f"Time: {context.timestamp}\n\n"
            f"## Market data\n{context.sentiment_summary}\n\n"
            f"## News headlines\n"
        )
        for n in context.news[:15]:
            line = f"- [{n.source}] {n.title}"
            if n.actual:
                line += f" (actual: {n.actual}, forecast: {n.forecast})"
            context_text += line + "\n"

        context_text += recent_text
        if skills_text:
            context_text += f"\n## Trading strategies (skills)\n{skills_text}"

        system = SYSTEM_PROMPT.format(context=context_text)

        # 5. Agent loop — call LLM, execute tools, repeat until done
        messages = [{"role": "user", "content": "Analyze the current market and decide whether to trade. Follow your process: OBSERVE → RESEARCH → ANALYZE → DECIDE → LOG → ACT."}]

        self.session.append("system", system[:500] + "...")
        self.session.append("user", messages[0]["content"])

        max_iterations = 15
        for i in range(max_iterations):
            print(f"[BRAIN] LLM call #{i+1}...")
            response = await self.llm.call(
                system=system,
                messages=messages,
                tools=self.tools.schemas(),
                max_tokens=self.config.max_tokens,
            )
            print(f"[BRAIN] Model: {response.model} | Tools: {len(response.tool_calls)} | Text: {len(response.content)} chars")

            # If no tool calls, agent is done thinking
            if not response.tool_calls:
                self.session.append("assistant", response.content)
                print(f"[BRAIN] Final: {response.content[:200]}")
                return response.content

            # Execute tool calls
            # Build assistant message with tool_use blocks (Anthropic format)
            assistant_content = []
            if response.content:
                assistant_content.append({"type": "text", "text": response.content})
            for tc in response.tool_calls:
                assistant_content.append({
                    "type": "tool_use",
                    "id": tc["id"],
                    "name": tc["name"],
                    "input": tc["input"],
                })
            messages.append({"role": "assistant", "content": assistant_content})

            # Execute and collect results
            tool_results_content = []
            last_logged_confidence = self._last_confidence
            for tc in response.tool_calls:
                print(f"  [TOOL] {tc['name']}({json.dumps(tc['input'], ensure_ascii=False)[:100]})")

                # SECURITY: Enforce confidence threshold in CODE, not just prompt.
                # Block trade execution if no log_decision was called or confidence < 70.
                if tc["name"] == "trade":
                    if last_logged_confidence < 70:
                        result = ToolResult(
                            name="trade",
                            result=f"BLOCKED: confidence {last_logged_confidence} < 70. "
                                   f"Call log_decision with confidence >= 70 before trading.",
                            success=False,
                        )
                        print(f"  [SECURITY] Trade blocked: confidence {last_logged_confidence} < 70")
                        tool_results_content.append({
                            "type": "tool_result",
                            "tool_use_id": tc["id"],
                            "content": result.result,
                        })
                        continue

                result = await self.tools.execute(tc["name"], tc["input"])
                print(f"  [TOOL] → {result.result[:100]}...")

                # Track confidence from log_decision calls
                if tc["name"] == "log_decision":
                    confidence = tc["input"].get("confidence", 0)
                    self._last_confidence = confidence
                    last_logged_confidence = confidence
                    self.decisions.log_decision(
                        symbol=tc["input"].get("symbol", ""),
                        action=tc["input"].get("action", ""),
                        confidence=confidence,
                        reasoning=tc["input"].get("reasoning", ""),
                        sources=tc["input"].get("sources", []),
                    )

                self.session.append("tool", result.result, tool=tc["name"])
                tool_results_content.append({
                    "type": "tool_result",
                    "tool_use_id": tc["id"],
                    "content": result.result,
                })
            messages.append({"role": "user", "content": tool_results_content})

        return "Max iterations reached. Agent stopped."

    async def run_loop(self):
        """Run continuously with configurable interval."""
        print(f"[BRAIN] Starting autonomous loop (every {self.config.loop_interval_minutes} min)")
        print(f"[BRAIN] Gateway: {self.config.gateway_url}")

        while True:
            try:
                result = await self.run_cycle()
                print(f"\n[BRAIN] Cycle complete: {result[:200]}")
            except KeyboardInterrupt:
                print("\n[BRAIN] Stopped by user.")
                break
            except Exception as e:
                print(f"\n[BRAIN] Error in cycle: {e}")

            print(f"[BRAIN] Next cycle in {self.config.loop_interval_minutes} minutes...")
            await asyncio.sleep(self.config.loop_interval_minutes * 60)
