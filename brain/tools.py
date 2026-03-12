"""Trading tools — functions the LLM can call during reasoning.
Each tool maps to a GreedyClaw REST API call or web action."""

import json
import ipaddress
from datetime import datetime, timezone
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


# ── Pure TA functions (no pandas/numpy dependency) ────────────────

def _sma(values: list[float], period: int) -> list[float | None]:
    """Simple Moving Average. Returns list same length as input; None where not enough data."""
    result: list[float | None] = [None] * len(values)
    if len(values) < period:
        return result
    window_sum = sum(values[:period])
    result[period - 1] = window_sum / period
    for i in range(period, len(values)):
        window_sum += values[i] - values[i - period]
        result[i] = window_sum / period
    return result


def _atr(highs: list[float], lows: list[float], closes: list[float], period: int = 14) -> list[float | None]:
    """Average True Range (Wilder's smoothing). Returns list same length as input."""
    n = len(highs)
    result: list[float | None] = [None] * n
    if n < 2:
        return result

    # True ranges
    tr = [highs[0] - lows[0]]
    for i in range(1, n):
        tr.append(max(
            highs[i] - lows[i],
            abs(highs[i] - closes[i - 1]),
            abs(lows[i] - closes[i - 1]),
        ))

    if n < period:
        return result

    # Initial ATR = simple average of first `period` TRs
    atr_val = sum(tr[:period]) / period
    result[period - 1] = atr_val
    for i in range(period, n):
        atr_val = (atr_val * (period - 1) + tr[i]) / period
        result[i] = atr_val

    return result


def _rsi(closes: list[float], period: int = 14) -> list[float | None]:
    """Relative Strength Index (Wilder's smoothing)."""
    n = len(closes)
    result: list[float | None] = [None] * n
    if n < period + 1:
        return result

    # Price changes
    deltas = [closes[i] - closes[i - 1] for i in range(1, n)]

    # Initial avg gain/loss
    gains = [max(d, 0.0) for d in deltas[:period]]
    losses = [max(-d, 0.0) for d in deltas[:period]]
    avg_gain = sum(gains) / period
    avg_loss = sum(losses) / period

    if avg_loss == 0:
        result[period] = 100.0
    else:
        rs = avg_gain / avg_loss
        result[period] = 100.0 - 100.0 / (1.0 + rs)

    for i in range(period, len(deltas)):
        delta = deltas[i]
        avg_gain = (avg_gain * (period - 1) + max(delta, 0.0)) / period
        avg_loss = (avg_loss * (period - 1) + max(-delta, 0.0)) / period
        if avg_loss == 0:
            result[i + 1] = 100.0
        else:
            rs = avg_gain / avg_loss
            result[i + 1] = 100.0 - 100.0 / (1.0 + rs)

    return result


def _swing_levels(highs: list[float], lows: list[float], lookback: int = 5) -> tuple[list[float], list[float]]:
    """Find swing highs (resistance) and swing lows (support) from recent price data."""
    resistances: list[float] = []
    supports: list[float] = []
    n = len(highs)
    for i in range(lookback, n - lookback):
        # Swing high: higher than `lookback` bars on each side
        if all(highs[i] >= highs[i - j] for j in range(1, lookback + 1)) and \
           all(highs[i] >= highs[i + j] for j in range(1, lookback + 1)):
            resistances.append(highs[i])
        # Swing low
        if all(lows[i] <= lows[i - j] for j in range(1, lookback + 1)) and \
           all(lows[i] <= lows[i + j] for j in range(1, lookback + 1)):
            supports.append(lows[i])
    return resistances, supports


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
            {
                "name": "get_ohlc",
                "description": (
                    "Fetch OHLCV candle data for a symbol with technical indicators. "
                    "Returns a table of candles (timestamp, O, H, L, C, volume) plus "
                    "current ATR(14), RSI(14), SMA(20), and SMA(50). "
                    "Essential for technical analysis before any trade decision."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Trading pair (XAUUSD, BTCUSDT, EURUSD)"},
                        "timeframe": {
                            "type": "string",
                            "description": "Candle timeframe: 1m, 5m, 15m, 1h, 4h, 1d",
                            "default": "1h",
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Number of candles to fetch (max 1000)",
                            "default": 100,
                        },
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "get_market_summary",
                "description": (
                    "Quick market snapshot for a symbol: current price, 24h change%, "
                    "ATR(14), RSI(14), trend direction (SMA20 vs SMA50), volatility regime "
                    "(low/normal/high), and support/resistance levels. "
                    "Use this for a fast overview before deeper analysis."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Trading pair (XAUUSD, BTCUSDT, EURUSD)"},
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "get_trade_history",
                "description": (
                    "Get recent completed trades with P&L from the audit log. "
                    "Use to review past performance, win rate, and recent results."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of recent trades to return (default 20)",
                            "default": 20,
                        },
                    },
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
            elif name == "get_ohlc":
                return await self._get_ohlc(input_data)
            elif name == "get_market_summary":
                return await self._get_market_summary(input_data)
            elif name == "get_trade_history":
                return await self._get_trade_history(input_data)
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

    async def _fetch_candles(self, symbol: str, timeframe: str = "1h", limit: int = 100) -> list[dict]:
        """Fetch raw candle data from gateway. Returns list of candle dicts."""
        resp = await self._client.get(
            f"{self.gateway_url}/ohlc/{symbol}",
            params={"timeframe": timeframe, "limit": limit},
        )
        resp.raise_for_status()
        body = resp.json()
        return body.get("candles", [])

    async def _get_ohlc(self, data: dict) -> ToolResult:
        symbol = data["symbol"].upper()
        timeframe = data.get("timeframe", "1h")
        limit = min(int(data.get("limit", 100)), 1000)

        candles = await self._fetch_candles(symbol, timeframe, limit)
        if not candles:
            return ToolResult(name="get_ohlc", result=f"No candle data for {symbol}", success=False)

        closes = [c["close"] for c in candles]
        highs = [c["high"] for c in candles]
        lows = [c["low"] for c in candles]

        # Calculate indicators
        atr_vals = _atr(highs, lows, closes, 14)
        rsi_vals = _rsi(closes, 14)
        sma20 = _sma(closes, 20)
        sma50 = _sma(closes, 50)

        # Format candle table (last 30 for readability)
        display_candles = candles[-30:]
        lines = [f"{'Timestamp':<20} {'Open':>10} {'High':>10} {'Low':>10} {'Close':>10} {'Volume':>12}"]
        lines.append("-" * 82)
        for c in display_candles:
            ts = datetime.fromtimestamp(c["timestamp"], tz=timezone.utc).strftime("%Y-%m-%d %H:%M")
            lines.append(
                f"{ts:<20} {c['open']:>10.2f} {c['high']:>10.2f} {c['low']:>10.2f} "
                f"{c['close']:>10.2f} {c['volume']:>12.2f}"
            )

        # Current indicator values (latest non-None)
        current_atr = next((v for v in reversed(atr_vals) if v is not None), None)
        current_rsi = next((v for v in reversed(rsi_vals) if v is not None), None)
        current_sma20 = next((v for v in reversed(sma20) if v is not None), None)
        current_sma50 = next((v for v in reversed(sma50) if v is not None), None)

        lines.append("")
        lines.append(f"--- Indicators ({len(candles)} candles, {timeframe}) ---")
        lines.append(f"ATR(14):  {current_atr:.4f}" if current_atr else "ATR(14):  N/A (need 14+ candles)")
        lines.append(f"RSI(14):  {current_rsi:.2f}" if current_rsi else "RSI(14):  N/A (need 15+ candles)")
        lines.append(f"SMA(20):  {current_sma20:.2f}" if current_sma20 else "SMA(20):  N/A (need 20+ candles)")
        lines.append(f"SMA(50):  {current_sma50:.2f}" if current_sma50 else "SMA(50):  N/A (need 50+ candles)")

        return ToolResult(name="get_ohlc", result="\n".join(lines))

    async def _get_market_summary(self, data: dict) -> ToolResult:
        symbol = data["symbol"].upper()

        candles = await self._fetch_candles(symbol, "1h", 50)
        if not candles:
            return ToolResult(name="get_market_summary", result=f"No data for {symbol}", success=False)

        closes = [c["close"] for c in candles]
        highs = [c["high"] for c in candles]
        lows = [c["low"] for c in candles]
        current_price = closes[-1]

        # 24h change (last 24 candles on 1h timeframe)
        bars_24h = min(24, len(closes) - 1)
        price_24h_ago = closes[-(bars_24h + 1)] if bars_24h > 0 else closes[0]
        change_24h = ((current_price - price_24h_ago) / price_24h_ago) * 100 if price_24h_ago else 0

        # Indicators
        atr_vals = _atr(highs, lows, closes, 14)
        rsi_vals = _rsi(closes, 14)
        sma20 = _sma(closes, 20)
        sma50 = _sma(closes, 50)

        current_atr = next((v for v in reversed(atr_vals) if v is not None), None)
        current_rsi = next((v for v in reversed(rsi_vals) if v is not None), None)
        current_sma20 = next((v for v in reversed(sma20) if v is not None), None)
        current_sma50 = next((v for v in reversed(sma50) if v is not None), None)

        # Trend direction
        if current_sma20 and current_sma50:
            if current_sma20 > current_sma50:
                trend = "BULLISH (SMA20 > SMA50)"
            elif current_sma20 < current_sma50:
                trend = "BEARISH (SMA20 < SMA50)"
            else:
                trend = "NEUTRAL"
        else:
            trend = "UNKNOWN (insufficient data)"

        # Volatility regime based on ATR percentile
        non_none_atrs = [v for v in atr_vals if v is not None]
        if current_atr and len(non_none_atrs) >= 5:
            sorted_atrs = sorted(non_none_atrs)
            rank = sum(1 for a in sorted_atrs if a <= current_atr)
            percentile = rank / len(sorted_atrs) * 100
            if percentile < 25:
                vol_regime = f"LOW (ATR percentile: {percentile:.0f}%)"
            elif percentile < 75:
                vol_regime = f"NORMAL (ATR percentile: {percentile:.0f}%)"
            else:
                vol_regime = f"HIGH (ATR percentile: {percentile:.0f}%)"
        else:
            vol_regime = "UNKNOWN"

        # Support/resistance
        resistances, supports = _swing_levels(highs, lows, lookback=3)
        # Keep closest levels
        res_levels = sorted(set(r for r in resistances if r > current_price))[:3]
        sup_levels = sorted(set(s for s in supports if s < current_price), reverse=True)[:3]

        lines = [
            f"=== Market Summary: {symbol} ===",
            f"Price:        {current_price:.2f}",
            f"24h Change:   {change_24h:+.2f}%",
            f"ATR(14):      {current_atr:.4f}" if current_atr else "ATR(14):      N/A",
            f"RSI(14):      {current_rsi:.2f}" if current_rsi else "RSI(14):      N/A",
            f"Trend:        {trend}",
            f"Volatility:   {vol_regime}",
            f"Resistance:   {', '.join(f'{r:.2f}' for r in res_levels)}" if res_levels else "Resistance:   none detected",
            f"Support:      {', '.join(f'{s:.2f}' for s in sup_levels)}" if sup_levels else "Support:      none detected",
        ]

        return ToolResult(name="get_market_summary", result="\n".join(lines))

    async def _get_trade_history(self, data: dict) -> ToolResult:
        limit = int(data.get("limit", 20))
        resp = await self._client.get(
            f"{self.gateway_url}/trades",
            params={"limit": limit},
        )
        body = resp.json()
        trades = body.get("trades", [])

        if not trades:
            return ToolResult(name="get_trade_history", result="No trade history found.")

        lines = [f"{'Time':<20} {'Symbol':<10} {'Side':<5} {'Qty':>8} {'Price':>10} {'PnL':>10} {'Status':<10}"]
        lines.append("-" * 83)
        for t in trades[:limit]:
            ts = t.get("timestamp", t.get("time", ""))[:19]
            symbol = t.get("symbol", "?")
            side = t.get("side", "?")
            qty = t.get("filled_qty", t.get("quantity", t.get("qty", 0)))
            price = t.get("avg_price", t.get("price", 0))
            pnl = t.get("pnl", t.get("realized_pnl", ""))
            status = t.get("status", "?")
            pnl_str = f"{pnl:+.2f}" if isinstance(pnl, (int, float)) else str(pnl)
            lines.append(
                f"{ts:<20} {symbol:<10} {str(side).upper():<5} {qty:>8.4f} {price:>10.2f} {pnl_str:>10} {status:<10}"
            )

        return ToolResult(name="get_trade_history", result="\n".join(lines))

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
