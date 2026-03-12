"""GreedyClaw Brain — entry point.

Usage:
  python -m brain                 # Run one analysis cycle
  python -m brain --loop          # Run continuously (every N minutes)
  python -m brain --setup         # Interactive onboarding wizard
  python -m brain --status        # Show config and provider status
"""

import asyncio
import sys
import os
from pathlib import Path


def setup_wizard():
    """Interactive onboarding for new users — zero-to-trading in 2 minutes."""
    from .config import BrainConfig, DEFAULT_BRAIN_YAML, _config_dir

    print()
    print("  GreedyClaw Brain — Setup Wizard")
    print("  Autonomous AI Trading Agent")
    print("  " + "=" * 40)
    print()

    config_dir = _config_dir()
    config_dir.mkdir(parents=True, exist_ok=True)

    # Step 1: LLM Provider
    print("  STEP 1: LLM Provider")
    print("  You need at least ONE AI provider API key.")
    print("  Supported: Anthropic, OpenAI, Google, DeepSeek, OpenRouter, Ollama")
    print()

    providers_found = []
    provider_keys = {
        "ANTHROPIC_API_KEY": "Anthropic (Claude)",
        "OPENAI_API_KEY": "OpenAI (GPT)",
        "GOOGLE_API_KEY": "Google (Gemini)",
        "DEEPSEEK_API_KEY": "DeepSeek",
        "OPENROUTER_API_KEY": "OpenRouter (200+ models)",
    }

    env_lines = []
    for env_key, label in provider_keys.items():
        existing = os.environ.get(env_key, "")
        if existing:
            providers_found.append(label)
            print(f"  [OK] {label}: configured")
        else:
            val = input(f"  {label} API key (Enter to skip): ").strip()
            if val:
                env_lines.append(f"{env_key}={val}")
                providers_found.append(label)

    ollama_url = os.environ.get("OLLAMA_URL", "")
    if ollama_url:
        providers_found.append("Ollama (local)")
        print(f"  [OK] Ollama: {ollama_url}")
    else:
        val = input("  Ollama URL (e.g. http://localhost:11434, Enter to skip): ").strip()
        if val:
            env_lines.append(f"OLLAMA_URL={val}")
            providers_found.append("Ollama (local)")

    if not providers_found:
        print("\n  ERROR: At least one LLM provider is required!")
        print("  Get a free key at: https://console.anthropic.com/")
        sys.exit(1)

    print(f"\n  Providers: {', '.join(providers_found)}")

    # Step 2: GreedyClaw Gateway
    print("\n  STEP 2: GreedyClaw Gateway")
    gw_url = input("  Gateway URL [http://127.0.0.1:7878]: ").strip() or "http://127.0.0.1:7878"
    gw_token = os.environ.get("GREEDYCLAW_AUTH_TOKEN", "")
    if not gw_token:
        gw_token = input("  Gateway auth token: ").strip()
    if gw_token:
        env_lines.append(f"GREEDYCLAW_AUTH_TOKEN={gw_token}")
    env_lines.append(f"GREEDYCLAW_URL={gw_url}")

    # Step 3: Market
    print("\n  STEP 3: Market Selection")
    print("  1. Forex / Gold (XAUUSD, EURUSD) — via MT5")
    print("  2. Crypto CEX (BTCUSDT, ETHUSDT) — via Binance/CCXT")
    print("  3. Solana DEX (PumpFun memecoins)")
    choice = input("  Choose [1]: ").strip() or "1"

    market_map = {"1": "forex", "2": "crypto", "3": "solana"}
    symbol_map = {"1": ["XAUUSD"], "2": ["BTCUSDT", "ETHUSDT"], "3": ["SOL"]}
    market = market_map.get(choice, "forex")
    symbols = symbol_map.get(choice, ["XAUUSD"])

    custom = input(f"  Symbols [{', '.join(symbols)}]: ").strip()
    if custom:
        symbols = [s.strip() for s in custom.split(",")]

    # Step 4: Interval
    print("\n  STEP 4: Agent Behavior")
    interval = input("  Check interval in minutes [15]: ").strip() or "15"

    # Save brain.yaml
    yaml_path = config_dir / "brain.yaml"
    yaml_content = (
        f"# GreedyClaw Brain config\n"
        f"model: claude-sonnet-4-20250514\n"
        f"loop_interval_minutes: {interval}\n"
        f"market: {market}\n"
        f"symbols:\n"
    )
    for s in symbols:
        yaml_content += f"  - {s}\n"
    yaml_content += (
        f"sources:\n"
        f"  - forex_factory\n"
        f"  - investing_com\n"
    )

    yaml_path.write_text(yaml_content, encoding="utf-8")
    print(f"\n  Saved: {yaml_path}")

    # Append to .env
    if env_lines:
        env_path = config_dir / ".env"
        existing_env = env_path.read_text(encoding="utf-8") if env_path.exists() else ""
        with open(env_path, "a", encoding="utf-8") as f:
            f.write("\n# Brain config (added by setup wizard)\n")
            for line in env_lines:
                key = line.split("=")[0]
                if key not in existing_env:
                    f.write(line + "\n")
        print(f"  Saved: {env_path}")

    # Done
    print("\n  " + "=" * 40)
    print("  Setup complete!")
    print()
    print("  Quick start:")
    print("    1. Start GreedyClaw:  greedyclaw serve")
    print("    2. Run Brain once:    python -m brain")
    print("    3. Run Brain loop:    python -m brain --loop")
    print()
    print(f"  Market: {market} | Symbols: {', '.join(symbols)}")
    print(f"  Interval: {interval} min | Providers: {', '.join(providers_found)}")
    print()


def show_status():
    """Show current config and provider status."""
    from .config import BrainConfig
    from .llm import _load_providers

    config = BrainConfig.load()
    providers = _load_providers()

    print()
    print("  GreedyClaw Brain — Status")
    print("  " + "=" * 40)
    print(f"  Market:     {config.market}")
    print(f"  Symbols:    {', '.join(config.symbols)}")
    print(f"  Interval:   {config.loop_interval_minutes} min")
    print(f"  Gateway:    {config.gateway_url}")
    print(f"  Data dir:   {config.data_dir}")
    print()

    print("  LLM Providers:")
    if not providers:
        print("    NONE configured! Run: python -m brain --setup")
    for i, p in enumerate(providers):
        marker = "PRIMARY" if i == 0 else f"fallback #{i}"
        print(f"    [{marker}] {p.name}/{p.model}")

    # Check gateway
    print()
    try:
        import httpx
        resp = httpx.get(
            f"{config.gateway_url}/status",
            headers={"Authorization": f"Bearer {config.gateway_token}"},
            timeout=5,
        )
        if resp.status_code == 200:
            print("  Gateway:    ONLINE")
        else:
            print(f"  Gateway:    ERROR ({resp.status_code})")
    except Exception:
        print("  Gateway:    OFFLINE — start with 'greedyclaw serve'")
    print()


def main():
    # Load .env from ~/.greedyclaw/.env
    env_path = Path.home() / ".greedyclaw" / ".env"
    if env_path.exists():
        for line in env_path.read_text(encoding="utf-8").split("\n"):
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, _, val = line.partition("=")
                os.environ.setdefault(key.strip(), val.strip())

    args = sys.argv[1:]

    if "--setup" in args or "--init" in args:
        setup_wizard()
        return

    if "--status" in args:
        show_status()
        return

    if "--help" in args or "-h" in args:
        print(__doc__)
        return

    from .config import BrainConfig
    from .agent import TradingAgent

    config = BrainConfig.load()
    agent = TradingAgent(config)

    if "--loop" in args:
        asyncio.run(agent.run_loop())
    else:
        result = asyncio.run(agent.run_cycle())
        print(f"\nResult: {result}")


if __name__ == "__main__":
    main()
