"""
CCXT Bridge — FastAPI sidecar wrapping CCXT for 100+ exchanges.
GreedyClaw calls this bridge via HTTP to trade on any CCXT-supported exchange.

Supports: Bybit, OKX, Kraken, Coinbase, Gate.io, KuCoin, Bitget, MEXC, HTX, and 100+ more.

Usage:
    pip install ccxt fastapi uvicorn pydantic
    python ccxt_bridge.py --exchange bybit --port 7880

Environment variables (per exchange):
    CCXT_API_KEY=...
    CCXT_SECRET=...
    CCXT_PASSWORD=...     (if exchange requires passphrase, e.g. OKX, KuCoin)
    CCXT_SANDBOX=true     (enable testnet/sandbox mode)
"""

import argparse
import logging
import os
from contextlib import asynccontextmanager
from datetime import datetime, timezone

import ccxt
from fastapi import FastAPI, HTTPException, Query
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO, format="%(asctime)s [CCXT-BRIDGE] %(message)s")
log = logging.getLogger("ccxt_bridge")

# ── Models ──────────────────────────────────────────────────────────

class OrderRequest(BaseModel):
    symbol: str
    side: str  # "buy" or "sell"
    order_type: str = "market"
    quantity: float
    price: float | None = None
    client_order_id: str = ""

class OrderResult(BaseModel):
    exchange_order_id: str
    client_order_id: str
    symbol: str
    side: str
    filled_qty: float
    avg_price: float
    status: str
    timestamp: str
    commission: float

# ── Exchange singleton ──────────────────────────────────────────────

exchange_instance: ccxt.Exchange | None = None
exchange_name: str = ""

def get_exchange() -> ccxt.Exchange:
    if exchange_instance is None:
        raise HTTPException(502, "Exchange not initialized")
    return exchange_instance

def create_exchange(name: str) -> ccxt.Exchange:
    """Create and configure a CCXT exchange instance."""
    exchange_class = getattr(ccxt, name, None)
    if exchange_class is None:
        raise ValueError(f"Unknown exchange: {name}. Available: {', '.join(ccxt.exchanges[:20])}...")

    config = {
        "apiKey": os.environ.get("CCXT_API_KEY", ""),
        "secret": os.environ.get("CCXT_SECRET", ""),
        "enableRateLimit": True,
        "timeout": 30000,
    }

    password = os.environ.get("CCXT_PASSWORD")
    if password:
        config["password"] = password

    ex = exchange_class(config)

    # Sandbox/testnet mode
    if os.environ.get("CCXT_SANDBOX", "").lower() in ("true", "1", "yes"):
        if hasattr(ex, "set_sandbox_mode"):
            ex.set_sandbox_mode(True)
            log.info("Sandbox/testnet mode enabled")

    return ex

# ── Lifespan ────────────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    global exchange_instance
    ex = get_exchange()
    try:
        markets = ex.load_markets()
        log.info("Loaded %d markets from %s", len(markets), ex.id)
    except Exception as e:
        log.warning("Could not load markets: %s", e)
    yield

app = FastAPI(title="CCXT Bridge", version="1.0.0", lifespan=lifespan)

# ── Health ──────────────────────────────────────────────────────────

@app.get("/health")
def health():
    ex = get_exchange()
    return {
        "status": "ok",
        "exchange": ex.id,
        "name": ex.name,
        "sandbox": getattr(ex, "sandbox", False),
        "markets_loaded": len(ex.markets) if ex.markets else 0,
    }

# ── Supported exchanges list ───────────────────────────────────────

@app.get("/exchanges")
def list_exchanges():
    return {"count": len(ccxt.exchanges), "exchanges": ccxt.exchanges}

# ── Account / Balance ──────────────────────────────────────────────

@app.get("/account")
def get_account():
    ex = get_exchange()
    try:
        balance = ex.fetch_balance()
    except Exception as e:
        raise HTTPException(502, f"Balance fetch failed: {e}")

    total = balance.get("total", {})
    free = balance.get("free", {})

    # Calculate USD-equivalent total (rough)
    total_usd = 0.0
    assets = []
    for asset, amount in total.items():
        if amount and amount > 0:
            free_amount = free.get(asset, 0) or 0
            locked = amount - free_amount
            assets.append({
                "asset": asset,
                "free": free_amount,
                "locked": locked,
            })
            # Rough USD conversion
            if asset in ("USD", "USDT", "USDC", "BUSD", "UST", "TUSD", "DAI"):
                total_usd += amount

    return {
        "total_usd": total_usd,
        "available_usd": sum(free.get(s, 0) or 0 for s in ("USD", "USDT", "USDC", "BUSD")),
        "assets": assets,
    }

# ── Price ───────────────────────────────────────────────────────────

@app.get("/price/{symbol:path}")
def get_price(symbol: str):
    ex = get_exchange()
    # Normalize: BTCUSDT → BTC/USDT
    normalized = normalize_symbol(symbol, ex)
    try:
        ticker = ex.fetch_ticker(normalized)
    except Exception as e:
        raise HTTPException(404, f"Price fetch failed for '{normalized}': {e}")

    return {
        "symbol": normalized,
        "bid": ticker.get("bid") or 0,
        "ask": ticker.get("ask") or 0,
        "last": ticker.get("last") or 0,
        "volume_24h": ticker.get("quoteVolume") or 0,
        "change_pct": ticker.get("percentage") or 0,
        "time": ticker.get("datetime") or datetime.now(tz=timezone.utc).isoformat(),
    }

# ── Positions (for futures/margin) ──────────────────────────────────

@app.get("/positions")
def get_positions(symbol: str | None = None):
    ex = get_exchange()
    if not hasattr(ex, "fetch_positions"):
        return []
    try:
        symbols = [normalize_symbol(symbol, ex)] if symbol else None
        positions = ex.fetch_positions(symbols)
    except Exception as e:
        log.warning("Positions fetch failed: %s", e)
        return []

    result = []
    for p in positions:
        side = p.get("side", "long")
        contracts = abs(float(p.get("contracts", 0) or 0))
        if contracts == 0:
            continue
        result.append({
            "symbol": p.get("symbol", ""),
            "side": side,
            "quantity": contracts,
            "avg_entry_price": float(p.get("entryPrice", 0) or 0),
            "current_price": float(p.get("markPrice", 0) or 0),
            "unrealized_pnl": float(p.get("unrealizedPnl", 0) or 0),
            "leverage": p.get("leverage"),
            "liquidation_price": p.get("liquidationPrice"),
        })
    return result

# ── Order ───────────────────────────────────────────────────────────

@app.post("/order")
def place_order(req: OrderRequest) -> OrderResult:
    ex = get_exchange()
    symbol = normalize_symbol(req.symbol, ex)
    side = req.side.lower()

    if side not in ("buy", "sell"):
        raise HTTPException(400, f"Invalid side: '{req.side}'")

    params = {}
    if req.client_order_id:
        params["clientOrderId"] = req.client_order_id

    try:
        if req.order_type.lower() == "market":
            order = ex.create_order(symbol, "market", side, req.quantity, params=params)
        elif req.order_type.lower() == "limit":
            if req.price is None:
                raise HTTPException(400, "Limit orders require a price")
            order = ex.create_order(symbol, "limit", side, req.quantity, req.price, params=params)
        else:
            raise HTTPException(400, f"Unsupported order type: '{req.order_type}'")
    except ccxt.InsufficientFunds as e:
        raise HTTPException(400, f"Insufficient funds: {e}")
    except ccxt.InvalidOrder as e:
        raise HTTPException(400, f"Invalid order: {e}")
    except ccxt.ExchangeError as e:
        raise HTTPException(502, f"Exchange error: {e}")
    except Exception as e:
        raise HTTPException(502, f"Order failed: {e}")

    log.info("Order placed: %s %s %s %.6f @ %s → %s",
             side, symbol, req.order_type, req.quantity,
             order.get("average") or order.get("price") or "market",
             order.get("id"))

    # Map status
    raw_status = order.get("status", "")
    status_map = {"closed": "Filled", "open": "New", "canceled": "Cancelled", "expired": "Expired"}
    status = status_map.get(raw_status, raw_status.capitalize())

    filled = float(order.get("filled", 0) or 0)
    avg_price = float(order.get("average", 0) or order.get("price", 0) or 0)

    # Calculate commission from fees
    fee = order.get("fee") or {}
    commission = float(fee.get("cost", 0) or 0)

    return OrderResult(
        exchange_order_id=str(order.get("id", "")),
        client_order_id=req.client_order_id or order.get("clientOrderId", ""),
        symbol=symbol,
        side=side,
        filled_qty=filled,
        avg_price=avg_price,
        status=status,
        timestamp=order.get("datetime") or datetime.now(tz=timezone.utc).isoformat(),
        commission=commission,
    )

# ── Cancel order ────────────────────────────────────────────────────

@app.delete("/order/{order_id}")
def cancel_order(order_id: str, symbol: str = Query(default="")):
    ex = get_exchange()
    sym = normalize_symbol(symbol, ex) if symbol else None
    try:
        result = ex.cancel_order(order_id, sym)
    except Exception as e:
        raise HTTPException(502, f"Cancel failed: {e}")
    return {"success": True, "order_id": order_id}

# ── Open orders ─────────────────────────────────────────────────────

@app.get("/orders")
def get_orders(symbol: str | None = None):
    ex = get_exchange()
    try:
        sym = normalize_symbol(symbol, ex) if symbol else None
        orders = ex.fetch_open_orders(sym)
    except Exception as e:
        raise HTTPException(502, f"Orders fetch failed: {e}")
    return [{
        "id": o.get("id"),
        "symbol": o.get("symbol"),
        "side": o.get("side"),
        "type": o.get("type"),
        "price": o.get("price"),
        "amount": o.get("amount"),
        "filled": o.get("filled"),
        "status": o.get("status"),
        "timestamp": o.get("datetime"),
    } for o in orders]

# ── Markets ─────────────────────────────────────────────────────────

@app.get("/markets")
def get_markets(query: str = ""):
    ex = get_exchange()
    if not ex.markets:
        try:
            ex.load_markets()
        except Exception:
            pass
    markets = list(ex.markets.keys()) if ex.markets else []
    if query:
        q = query.upper()
        markets = [m for m in markets if q in m.upper()]
    return {"count": len(markets), "markets": markets[:100]}

# ── Symbol normalization ────────────────────────────────────────────

def normalize_symbol(symbol: str, ex: ccxt.Exchange) -> str:
    """Try to normalize symbol to CCXT format (e.g., BTCUSDT → BTC/USDT)."""
    if "/" in symbol:
        return symbol
    # Try common quote currencies
    for quote in ("USDT", "USD", "USDC", "BTC", "ETH", "BUSD", "EUR", "GBP"):
        if symbol.upper().endswith(quote):
            base = symbol[: len(symbol) - len(quote)]
            candidate = f"{base}/{quote}"
            if ex.markets and candidate in ex.markets:
                return candidate
    # Try as-is (some exchanges accept concatenated)
    if ex.markets and symbol in ex.markets:
        return symbol
    # Return with slash guess
    return symbol

# ── Run ─────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import uvicorn

    parser = argparse.ArgumentParser(description="CCXT Bridge for GreedyClaw")
    parser.add_argument("--exchange", required=True, help="Exchange ID (e.g., bybit, okx, kraken)")
    parser.add_argument("--port", type=int, default=7880, help="Port (default: 7880)")
    parser.add_argument("--host", default="127.0.0.1", help="Host (default: 127.0.0.1)")
    args = parser.parse_args()

    exchange_name = args.exchange
    exchange_instance = create_exchange(args.exchange)
    log.info("Starting CCXT Bridge for %s on %s:%d", exchange_instance.name, args.host, args.port)

    uvicorn.run(app, host=args.host, port=args.port, log_level="info")
