"""Trading tools — functions the LLM can call during reasoning.
Each tool maps to a GreedyClaw REST API call or web action."""

import json
import ipaddress
from urllib.parse import urlparse
import httpx
from dataclasses import dataclass


# SSRF protection — block internal/private IP ranges
_BLOCKED_NETWORKS = [
    ipaddress.ip_network("127.0.0.0/8"),      # loopback
    ipaddress.ip_network("10.0.0.0/8"),        # private class A
    ipaddress.ip_network("172.16.0.0/12"),     # private class B
    ipaddress.ip_network("192.168.0.0/16"),    # private class C
    ipaddress.ip_network("169.254.0.0/16"),    # link-local
    ipaddress.ip_network("0.0.0.0/8"),         # unspecified
    ipaddress.ip_network("::1/128"),           # IPv6 loopback
    ipaddress.ip_network("fc00::/7"),          # IPv6 private
    ipaddress.ip_network("fe80::/10"),         # IPv6 link-local
]


def _is_ssrf_safe(url: str) -> bool:
    """Check if URL is safe to fetch (not pointing to internal/private IPs)."""
    try:
        parsed = urlparse(url)
        hostname = parsed.hostname or ""
        # Block common internal hostnames
        if hostname in ("localhost", "metadata.google.internal", "169.254.169.254"):
            return False
        # Resolve and check IP
        import socket
        for info in socket.getaddrinfo(hostname, parsed.port or 443):
            ip = ipaddress.ip_address(info[4][0])
            for net in _BLOCKED_NETWORKS:
                if ip in net:
                    return False
        return True
    except Exception:
        return False


@dataclass
class ToolResult:
    name: str
    result: str
    success: bool = True


class TradingTools:
    """Executes tool calls from the LLM against GreedyClaw gateway."""

    def __init__(self, gateway_url: str, gateway_token: str):
        self.gateway_url = gateway_url.rstrip("/")
        self.headers = {
            "Authorization": f"Bearer {gateway_token}",
            "Content-Type": "application/json",
        }
        self._client = httpx.AsyncClient(timeout=30, headers=self.headers)

    # ── Tool schemas (for LLM) ───────────────────────────────────

    @staticmethod
    def schemas() -> list[dict]:
        return [
            {
                "name": "trade",
                "description": "Execute a buy/sell trade on the configured exchange via GreedyClaw. Returns fill details and risk snapshot. Use with caution — this places real orders.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {"type": "string", "enum": ["buy", "sell"], "description": "Trade direction"},
                        "symbol": {"type": "string", "description": "Trading pair (XAUUSD, BTCUSDT, EURUSD)"},
                        "amount": {"type": "number", "description": "Quantity (lot size for forex, base asset for crypto)"},
                    },
                    "required": ["action", "symbol", "amount"],
                },
            },
            {
                "name": "get_price",
                "description": "Get current bid/ask price for a symbol.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Trading pair"},
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "get_positions",
                "description": "Get all open positions with entry price, current price, and unrealized PnL.",
                "parameters": {"type": "object", "properties": {}},
            },
            {
                "name": "get_balance",
                "description": "Get account balance (total equity, available margin, asset breakdown).",
                "parameters": {"type": "object", "properties": {}},
            },
            {
                "name": "get_risk_status",
                "description": "Get risk engine status: daily PnL, exposure, remaining limits, rate limit state.",
                "parameters": {"type": "object", "properties": {}},
            },
            {
                "name": "web_search",
                "description": "Search the web for market news, economic events, or analysis. Use for research before trading.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query (e.g. 'gold price forecast today', 'Fed rate decision')"},
                    },
                    "required": ["query"],
                },
            },
            {
                "name": "fetch_url",
                "description": "Fetch and extract text content from a URL. Use for reading specific articles or data pages.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "URL to fetch"},
                    },
                    "required": ["url"],
                },
            },
            {
                "name": "log_decision",
                "description": "Log a trading decision with reasoning. ALWAYS call this before placing a trade to record your analysis.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string"},
                        "action": {"type": "string", "enum": ["buy", "sell", "hold", "close"]},
                        "confidence": {"type": "number", "description": "Confidence 0-100"},
                        "reasoning": {"type": "string", "description": "Detailed reasoning for this decision"},
                        "sources": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Sources used (URLs, data points)",
                        },
                    },
                    "required": ["symbol", "action", "confidence", "reasoning"],
                },
            },
        ]

    # ── Execute tool calls ───────────────────────────────────────

    async def execute(self, name: str, input_data: dict) -> ToolResult:
        try:
            if name == "trade":
                return await self._trade(input_data)
            elif name == "get_price":
                return await self._get_price(input_data)
            elif name == "get_positions":
                return await self._get(f"/positions")
            elif name == "get_balance":
                return await self._get(f"/balance")
            elif name == "get_risk_status":
                return await self._get(f"/status")
            elif name == "web_search":
                return await self._web_search(input_data)
            elif name == "fetch_url":
                return await self._fetch_url(input_data)
            elif name == "log_decision":
                return ToolResult(name="log_decision", result="Decision logged.", success=True)
            else:
                return ToolResult(name=name, result=f"Unknown tool: {name}", success=False)
        except Exception as e:
            return ToolResult(name=name, result=f"Error: {e}", success=False)

    async def _trade(self, data: dict) -> ToolResult:
        resp = await self._client.post(
            f"{self.gateway_url}/trade",
            json={"action": data["action"], "symbol": data["symbol"], "amount": data["amount"]},
        )
        body = resp.json()
        return ToolResult(name="trade", result=json.dumps(body, indent=2), success=body.get("success", False))

    async def _get_price(self, data: dict) -> ToolResult:
        resp = await self._client.get(f"{self.gateway_url}/price/{data['symbol']}")
        return ToolResult(name="get_price", result=resp.text)

    async def _get(self, path: str) -> ToolResult:
        resp = await self._client.get(f"{self.gateway_url}{path}")
        return ToolResult(name=path, result=resp.text)

    async def _web_search(self, data: dict) -> ToolResult:
        query = data["query"]
        # Use DuckDuckGo HTML (no API key needed)
        try:
            async with httpx.AsyncClient(timeout=10) as client:
                resp = await client.get(
                    "https://html.duckduckgo.com/html/",
                    params={"q": query},
                    headers={"User-Agent": "Mozilla/5.0"},
                )
                from bs4 import BeautifulSoup
                soup = BeautifulSoup(resp.text, "html.parser")
                results = []
                for r in soup.select(".result")[:8]:
                    title_el = r.select_one(".result__title")
                    snippet_el = r.select_one(".result__snippet")
                    if title_el:
                        results.append({
                            "title": title_el.get_text(strip=True),
                            "snippet": snippet_el.get_text(strip=True) if snippet_el else "",
                        })
                return ToolResult(name="web_search", result=json.dumps(results, indent=2))
        except Exception as e:
            return ToolResult(name="web_search", result=f"Search failed: {e}", success=False)

    async def _fetch_url(self, data: dict) -> ToolResult:
        url = data.get("url", "")
        # SSRF protection — block internal/private IPs
        if not _is_ssrf_safe(url):
            return ToolResult(
                name="fetch_url",
                result="Blocked: URL points to internal/private network",
                success=False,
            )
        try:
            import trafilatura
            async with httpx.AsyncClient(timeout=15) as client:
                resp = await client.get(url, headers={"User-Agent": "Mozilla/5.0"})
                text = trafilatura.extract(resp.text, include_comments=False) or resp.text[:3000]
                return ToolResult(name="fetch_url", result=text[:4000])
        except Exception as e:
            return ToolResult(name="fetch_url", result=f"Fetch failed: {e}", success=False)
