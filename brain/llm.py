"""Multi-provider LLM layer — Anthropic, OpenAI, Google Gemini, Ollama, DeepSeek.
Supports automatic failover, rotation on rate limits, and unified tool calling.
Inspired by OpenClaw's model-auth.ts pattern."""

import os
import json
import httpx
from dataclasses import dataclass
from typing import AsyncIterator


@dataclass
class LLMResponse:
    content: str
    tool_calls: list[dict]  # [{"name": ..., "input": ..., "id": ...}]
    model: str
    usage: dict


@dataclass
class ProviderConfig:
    name: str       # anthropic, openai, google, ollama, deepseek
    api_key: str
    model: str
    base_url: str = ""
    priority: int = 0  # lower = higher priority


def _load_providers() -> list[ProviderConfig]:
    """Load all available providers from env vars. More keys = more fallbacks."""
    providers = []

    # Anthropic
    key = os.environ.get("ANTHROPIC_API_KEY", "")
    if key:
        providers.append(ProviderConfig(
            name="anthropic", api_key=key,
            model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-20250514"),
            priority=0,
        ))

    # OpenAI
    key = os.environ.get("OPENAI_API_KEY", "")
    if key:
        providers.append(ProviderConfig(
            name="openai", api_key=key,
            model=os.environ.get("OPENAI_MODEL", "gpt-4o"),
            priority=1,
        ))

    # Google Gemini
    key = os.environ.get("GOOGLE_API_KEY", "")
    if key:
        providers.append(ProviderConfig(
            name="google", api_key=key,
            model=os.environ.get("GOOGLE_MODEL", "gemini-2.5-flash"),
            priority=2,
        ))

    # DeepSeek
    key = os.environ.get("DEEPSEEK_API_KEY", "")
    if key:
        providers.append(ProviderConfig(
            name="deepseek", api_key=key,
            model=os.environ.get("DEEPSEEK_MODEL", "deepseek-chat"),
            base_url="https://api.deepseek.com/v1",
            priority=3,
        ))

    # Ollama (local, no key needed)
    ollama_url = os.environ.get("OLLAMA_URL", "")
    if ollama_url:
        providers.append(ProviderConfig(
            name="ollama", api_key="",
            model=os.environ.get("OLLAMA_MODEL", "llama3.1:70b"),
            base_url=ollama_url,
            priority=10,
        ))

    # OpenRouter (aggregator — 200+ models)
    key = os.environ.get("OPENROUTER_API_KEY", "")
    if key:
        providers.append(ProviderConfig(
            name="openrouter", api_key=key,
            model=os.environ.get("OPENROUTER_MODEL", "anthropic/claude-sonnet-4-20250514"),
            base_url="https://openrouter.ai/api/v1",
            priority=5,
        ))

    providers.sort(key=lambda p: p.priority)
    return providers


class LLMRouter:
    """Routes LLM calls across providers with automatic failover."""

    def __init__(self):
        self.providers = _load_providers()
        if not self.providers:
            raise RuntimeError(
                "No LLM providers configured. Set at least one API key:\n"
                "  ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY, "
                "DEEPSEEK_API_KEY, OPENROUTER_API_KEY, or OLLAMA_URL"
            )
        self._client = httpx.AsyncClient(timeout=120)

    @property
    def provider_names(self) -> list[str]:
        return [p.name for p in self.providers]

    async def call(
        self,
        system: str,
        messages: list[dict],
        tools: list[dict] | None = None,
        max_tokens: int = 4096,
    ) -> LLMResponse:
        """Call the highest-priority available provider. Failover on errors."""
        last_error = None
        for provider in self.providers:
            try:
                return await self._call_provider(provider, system, messages, tools, max_tokens)
            except Exception as e:
                last_error = e
                print(f"[LLM] {provider.name}/{provider.model} failed: {e}, trying next...")
                continue
        raise RuntimeError(f"All LLM providers failed. Last error: {last_error}")

    async def _call_provider(
        self,
        provider: ProviderConfig,
        system: str,
        messages: list[dict],
        tools: list[dict] | None,
        max_tokens: int,
    ) -> LLMResponse:
        if provider.name == "anthropic":
            return await self._call_anthropic(provider, system, messages, tools, max_tokens)
        elif provider.name in ("openai", "deepseek", "openrouter"):
            return await self._call_openai_compat(provider, system, messages, tools, max_tokens)
        elif provider.name == "google":
            return await self._call_google(provider, system, messages, tools, max_tokens)
        elif provider.name == "ollama":
            return await self._call_ollama(provider, system, messages, tools, max_tokens)
        else:
            raise ValueError(f"Unknown provider: {provider.name}")

    # ── Anthropic ────────────────────────────────────────────────────

    async def _call_anthropic(self, p, system, messages, tools, max_tokens) -> LLMResponse:
        body = {
            "model": p.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": messages,
        }
        if tools:
            body["tools"] = [self._tool_to_anthropic(t) for t in tools]

        resp = await self._client.post(
            "https://api.anthropic.com/v1/messages",
            headers={
                "x-api-key": p.api_key,
                "anthropic-version": "2023-06-01",
                "content-type": "application/json",
            },
            json=body,
        )
        resp.raise_for_status()
        data = resp.json()

        content = ""
        tool_calls = []
        for block in data.get("content", []):
            if block["type"] == "text":
                content += block["text"]
            elif block["type"] == "tool_use":
                tool_calls.append({
                    "id": block["id"],
                    "name": block["name"],
                    "input": block["input"],
                })

        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            model=f"anthropic/{p.model}",
            usage=data.get("usage", {}),
        )

    def _tool_to_anthropic(self, tool: dict) -> dict:
        return {
            "name": tool["name"],
            "description": tool["description"],
            "input_schema": tool["parameters"],
        }

    # ── OpenAI-compatible (OpenAI, DeepSeek, OpenRouter) ─────────

    async def _call_openai_compat(self, p, system, messages, tools, max_tokens) -> LLMResponse:
        base = p.base_url or "https://api.openai.com/v1"
        oai_messages = [{"role": "system", "content": system}]
        for m in messages:
            oai_messages.append(m)

        body = {
            "model": p.model,
            "max_tokens": max_tokens,
            "messages": oai_messages,
        }
        if tools:
            body["tools"] = [{"type": "function", "function": t} for t in tools]

        headers = {"Authorization": f"Bearer {p.api_key}", "Content-Type": "application/json"}
        if p.name == "openrouter":
            headers["HTTP-Referer"] = "https://github.com/GreedyClaw/GreedyClaw"

        resp = await self._client.post(f"{base}/chat/completions", headers=headers, json=body)
        resp.raise_for_status()
        data = resp.json()

        choice = data["choices"][0]
        msg = choice["message"]
        content = msg.get("content", "") or ""
        tool_calls = []
        for tc in msg.get("tool_calls", []):
            tool_calls.append({
                "id": tc["id"],
                "name": tc["function"]["name"],
                "input": json.loads(tc["function"]["arguments"]),
            })

        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            model=f"{p.name}/{p.model}",
            usage=data.get("usage", {}),
        )

    # ── Google Gemini ────────────────────────────────────────────────

    async def _call_google(self, p, system, messages, tools, max_tokens) -> LLMResponse:
        # Convert to Gemini format
        contents = []
        for m in messages:
            role = "user" if m["role"] == "user" else "model"
            contents.append({"role": role, "parts": [{"text": m["content"]}]})

        body = {
            "contents": contents,
            "systemInstruction": {"parts": [{"text": system}]},
            "generationConfig": {"maxOutputTokens": max_tokens},
        }
        if tools:
            body["tools"] = [{"functionDeclarations": [
                {"name": t["name"], "description": t["description"], "parameters": t["parameters"]}
                for t in tools
            ]}]

        resp = await self._client.post(
            f"https://generativelanguage.googleapis.com/v1beta/models/{p.model}:generateContent?key={p.api_key}",
            json=body,
        )
        resp.raise_for_status()
        data = resp.json()

        content = ""
        tool_calls = []
        for candidate in data.get("candidates", []):
            for part in candidate.get("content", {}).get("parts", []):
                if "text" in part:
                    content += part["text"]
                if "functionCall" in part:
                    fc = part["functionCall"]
                    tool_calls.append({
                        "id": f"gemini-{fc['name']}",
                        "name": fc["name"],
                        "input": fc.get("args", {}),
                    })

        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            model=f"google/{p.model}",
            usage={},
        )

    # ── Ollama ───────────────────────────────────────────────────────

    async def _call_ollama(self, p, system, messages, tools, max_tokens) -> LLMResponse:
        oai_messages = [{"role": "system", "content": system}]
        oai_messages.extend(messages)

        body = {"model": p.model, "messages": oai_messages, "stream": False}
        if tools:
            body["tools"] = [{"type": "function", "function": t} for t in tools]

        resp = await self._client.post(f"{p.base_url}/api/chat", json=body)
        resp.raise_for_status()
        data = resp.json()

        msg = data.get("message", {})
        content = msg.get("content", "")
        tool_calls = []
        for tc in msg.get("tool_calls", []):
            fn = tc.get("function", {})
            tool_calls.append({
                "id": f"ollama-{fn.get('name', '')}",
                "name": fn.get("name", ""),
                "input": fn.get("arguments", {}),
            })

        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            model=f"ollama/{p.model}",
            usage={},
        )
