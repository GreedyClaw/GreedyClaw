"""
MT5 Bridge — FastAPI sidecar that wraps MetaTrader 5 Python API.
GreedyClaw calls this bridge via HTTP to execute trades on MT5.

Usage:
    pip install MetaTrader5 fastapi uvicorn
    python mt5_bridge.py [--port 7879] [--mt5-path "C:/..."]
"""

import argparse
import logging
import time
from contextlib import asynccontextmanager
from datetime import datetime, timezone

import MetaTrader5 as mt5
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO, format="%(asctime)s [MT5-BRIDGE] %(message)s")
log = logging.getLogger("mt5_bridge")

# ── Models ──────────────────────────────────────────────────────────

class OrderRequest(BaseModel):
    symbol: str
    side: str  # "buy" or "sell"
    order_type: str = "market"  # "market" or "limit"
    quantity: float
    price: float | None = None
    client_order_id: str = ""
    sl: float | None = None
    tp: float | None = None
    deviation: int = 20  # max slippage in points
    magic: int = 777777

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

class PositionInfo(BaseModel):
    ticket: int
    symbol: str
    side: str
    quantity: float
    avg_entry_price: float
    current_price: float
    unrealized_pnl: float
    sl: float
    tp: float
    magic: int
    comment: str
    open_time: str

class AccountInfo(BaseModel):
    total_usd: float
    available_usd: float
    equity: float
    margin: float
    margin_free: float
    margin_level: float
    leverage: int
    currency: str
    server: str
    name: str

class SymbolPrice(BaseModel):
    symbol: str
    bid: float
    ask: float
    last: float
    spread: int
    time: str

# ── MT5 connection ──────────────────────────────────────────────────

MT5_PATH: str | None = None

def ensure_mt5():
    """Ensure MT5 is initialized. Re-init if disconnected."""
    info = mt5.terminal_info()
    if info is None:
        kwargs = {}
        if MT5_PATH:
            kwargs["path"] = MT5_PATH
        if not mt5.initialize(**kwargs):
            raise HTTPException(status_code=502, detail=f"MT5 init failed: {mt5.last_error()}")
        log.info("MT5 connected: %s", mt5.terminal_info().name)

# ── Lifespan ────────────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    kwargs = {}
    if MT5_PATH:
        kwargs["path"] = MT5_PATH
    if not mt5.initialize(**kwargs):
        log.error("MT5 init failed: %s", mt5.last_error())
        raise RuntimeError(f"Cannot connect to MetaTrader 5: {mt5.last_error()}")
    info = mt5.terminal_info()
    acct = mt5.account_info()
    log.info("MT5 connected: %s", info.name)
    log.info("Account: %s (#%d) %s, Balance: %.2f %s",
             acct.name, acct.login, acct.server, acct.balance, acct.currency)
    yield
    mt5.shutdown()
    log.info("MT5 disconnected")

app = FastAPI(title="MT5 Bridge", version="1.0.0", lifespan=lifespan)

# ── Health ──────────────────────────────────────────────────────────

@app.get("/health")
def health():
    ensure_mt5()
    info = mt5.terminal_info()
    return {"status": "ok", "terminal": info.name, "connected": info.connected}

# ── Account ─────────────────────────────────────────────────────────

@app.get("/account")
def get_account() -> AccountInfo:
    ensure_mt5()
    a = mt5.account_info()
    if a is None:
        raise HTTPException(502, "Cannot get account info")
    return AccountInfo(
        total_usd=a.balance,
        available_usd=a.margin_free,
        equity=a.equity,
        margin=a.margin,
        margin_free=a.margin_free,
        margin_level=a.margin_level or 0.0,
        leverage=a.leverage,
        currency=a.currency,
        server=a.server,
        name=a.name,
    )

# ── Price ───────────────────────────────────────────────────────────

@app.get("/price/{symbol}")
def get_price(symbol: str) -> SymbolPrice:
    ensure_mt5()
    tick = mt5.symbol_info_tick(symbol)
    if tick is None:
        # Try enabling the symbol first
        mt5.symbol_select(symbol, True)
        time.sleep(0.1)
        tick = mt5.symbol_info_tick(symbol)
        if tick is None:
            raise HTTPException(404, f"Symbol '{symbol}' not found or no tick data")
    return SymbolPrice(
        symbol=symbol,
        bid=tick.bid,
        ask=tick.ask,
        last=tick.last,
        spread=int((tick.ask - tick.bid) / mt5.symbol_info(symbol).point),
        time=datetime.fromtimestamp(tick.time, tz=timezone.utc).isoformat(),
    )

# ── Positions ───────────────────────────────────────────────────────

@app.get("/positions")
def get_positions(symbol: str | None = None) -> list[PositionInfo]:
    ensure_mt5()
    if symbol:
        positions = mt5.positions_get(symbol=symbol)
    else:
        positions = mt5.positions_get()
    if positions is None:
        return []
    result = []
    for p in positions:
        result.append(PositionInfo(
            ticket=p.ticket,
            symbol=p.symbol,
            side="buy" if p.type == mt5.ORDER_TYPE_BUY else "sell",
            quantity=p.volume,
            avg_entry_price=p.price_open,
            current_price=p.price_current,
            unrealized_pnl=p.profit,
            sl=p.sl,
            tp=p.tp,
            magic=p.magic,
            comment=p.comment,
            open_time=datetime.fromtimestamp(p.time, tz=timezone.utc).isoformat(),
        ))
    return result

# ── Order ───────────────────────────────────────────────────────────

@app.post("/order")
def place_order(req: OrderRequest) -> OrderResult:
    ensure_mt5()

    # Enable symbol
    mt5.symbol_select(req.symbol, True)
    time.sleep(0.05)

    info = mt5.symbol_info(req.symbol)
    if info is None:
        raise HTTPException(400, f"Symbol '{req.symbol}' not found")
    if not info.visible:
        raise HTTPException(400, f"Symbol '{req.symbol}' not visible in Market Watch")

    tick = mt5.symbol_info_tick(req.symbol)
    if tick is None:
        raise HTTPException(502, f"No tick data for '{req.symbol}'")

    # Determine order type and price
    if req.side.lower() == "buy":
        mt5_type = mt5.ORDER_TYPE_BUY
        price = tick.ask
    elif req.side.lower() == "sell":
        mt5_type = mt5.ORDER_TYPE_SELL
        price = tick.bid
    else:
        raise HTTPException(400, f"Invalid side: '{req.side}'")

    if req.order_type.lower() == "limit":
        if req.price is None:
            raise HTTPException(400, "Limit orders require a price")
        mt5_type = mt5.ORDER_TYPE_BUY_LIMIT if req.side.lower() == "buy" else mt5.ORDER_TYPE_SELL_LIMIT
        price = req.price

    # Normalize volume to symbol's lot step
    lot_step = info.volume_step
    lot_min = info.volume_min
    lot_max = info.volume_max
    volume = max(lot_min, min(lot_max, round(req.quantity / lot_step) * lot_step))

    request = {
        "action": mt5.TRADE_ACTION_DEAL if req.order_type.lower() == "market" else mt5.TRADE_ACTION_PENDING,
        "symbol": req.symbol,
        "volume": volume,
        "type": mt5_type,
        "price": price,
        "deviation": req.deviation,
        "magic": req.magic,
        "comment": req.client_order_id or "GreedyClaw",
        "type_time": mt5.ORDER_TIME_GTC,
        "type_filling": mt5.ORDER_FILLING_IOC,
    }

    if req.sl is not None:
        request["sl"] = req.sl
    if req.tp is not None:
        request["tp"] = req.tp

    log.info("Sending order: %s %s %.4f %s @ %.5f", req.side, req.symbol, volume,
             req.order_type, price)

    result = mt5.order_send(request)
    if result is None:
        raise HTTPException(502, f"order_send returned None: {mt5.last_error()}")

    if result.retcode != mt5.TRADE_RETCODE_DONE:
        # Try FOK filling if IOC rejected
        if result.retcode == mt5.TRADE_RETCODE_INVALID_FILL:
            request["type_filling"] = mt5.ORDER_FILLING_FOK
            result = mt5.order_send(request)
            if result is None:
                raise HTTPException(502, f"order_send FOK returned None: {mt5.last_error()}")
        # Try RETURN filling
        if result.retcode != mt5.TRADE_RETCODE_DONE:
            if result.retcode == mt5.TRADE_RETCODE_INVALID_FILL:
                request["type_filling"] = mt5.ORDER_FILLING_RETURN
                result = mt5.order_send(request)
                if result is None:
                    raise HTTPException(502, f"order_send RETURN returned None")

    if result.retcode != mt5.TRADE_RETCODE_DONE:
        raise HTTPException(
            502,
            f"Order failed: {result.comment} (retcode={result.retcode})"
        )

    log.info("Order filled: ticket=%d, volume=%.4f, price=%.5f",
             result.order, result.volume, result.price)

    # Map status
    status = "Filled" if result.retcode == mt5.TRADE_RETCODE_DONE else "Rejected"

    return OrderResult(
        exchange_order_id=str(result.order),
        client_order_id=req.client_order_id,
        symbol=req.symbol,
        side=req.side.lower(),
        filled_qty=result.volume,
        avg_price=result.price,
        status=status,
        timestamp=datetime.now(tz=timezone.utc).isoformat(),
        commission=0.0,  # MT5 reports commission separately via deals
    )

# ── Close position ──────────────────────────────────────────────────

@app.delete("/position/{ticket}")
def close_position(ticket: int) -> OrderResult:
    ensure_mt5()

    positions = mt5.positions_get(ticket=ticket)
    if not positions:
        raise HTTPException(404, f"Position #{ticket} not found")

    pos = positions[0]
    symbol = pos.symbol
    tick = mt5.symbol_info_tick(symbol)
    if tick is None:
        raise HTTPException(502, f"No tick for {symbol}")

    # Close = opposite trade
    if pos.type == mt5.ORDER_TYPE_BUY:
        close_type = mt5.ORDER_TYPE_SELL
        price = tick.bid
        side = "sell"
    else:
        close_type = mt5.ORDER_TYPE_BUY
        price = tick.ask
        side = "buy"

    request = {
        "action": mt5.TRADE_ACTION_DEAL,
        "symbol": symbol,
        "volume": pos.volume,
        "type": close_type,
        "position": ticket,
        "price": price,
        "deviation": 20,
        "magic": pos.magic,
        "comment": f"gc-close-{ticket}",
        "type_time": mt5.ORDER_TIME_GTC,
        "type_filling": mt5.ORDER_FILLING_IOC,
    }

    result = mt5.order_send(request)
    if result is None:
        raise HTTPException(502, f"Close failed: {mt5.last_error()}")
    if result.retcode != mt5.TRADE_RETCODE_DONE:
        # Try FOK
        request["type_filling"] = mt5.ORDER_FILLING_FOK
        result = mt5.order_send(request)
        if result is None or result.retcode != mt5.TRADE_RETCODE_DONE:
            raise HTTPException(502, f"Close failed: {result.comment if result else 'None'}")

    log.info("Position #%d closed: %.4f @ %.5f", ticket, result.volume, result.price)

    return OrderResult(
        exchange_order_id=str(result.order),
        client_order_id=f"gc-close-{ticket}",
        symbol=symbol,
        side=side,
        filled_qty=result.volume,
        avg_price=result.price,
        status="Filled",
        timestamp=datetime.now(tz=timezone.utc).isoformat(),
        commission=0.0,
    )

# ── Cancel pending order ────────────────────────────────────────────

@app.delete("/order/{ticket}")
def cancel_order(ticket: int):
    ensure_mt5()

    request = {
        "action": mt5.TRADE_ACTION_REMOVE,
        "order": ticket,
    }
    result = mt5.order_send(request)
    if result is None:
        raise HTTPException(502, f"Cancel failed: {mt5.last_error()}")
    if result.retcode != mt5.TRADE_RETCODE_DONE:
        raise HTTPException(502, f"Cancel failed: {result.comment}")

    return {"success": True, "ticket": ticket}

# ── Symbols ─────────────────────────────────────────────────────────

@app.get("/symbols")
def list_symbols(group: str | None = None):
    ensure_mt5()
    if group:
        symbols = mt5.symbols_get(group=group)
    else:
        symbols = mt5.symbols_get()
    if symbols is None:
        return []
    return [{"name": s.name, "description": s.description, "path": s.path,
             "digits": s.digits, "lot_min": s.volume_min, "lot_max": s.volume_max,
             "lot_step": s.volume_step, "point": s.point, "spread": s.spread}
            for s in symbols[:200]]  # cap at 200

# ── Orders (pending) ───────────────────────────────────────────────

@app.get("/orders")
def get_orders(symbol: str | None = None):
    ensure_mt5()
    if symbol:
        orders = mt5.orders_get(symbol=symbol)
    else:
        orders = mt5.orders_get()
    if orders is None:
        return []
    return [{"ticket": o.ticket, "symbol": o.symbol, "type": o.type,
             "volume": o.volume_current, "price": o.price_open,
             "sl": o.sl, "tp": o.tp, "comment": o.comment}
            for o in orders]

# ── Modify position SL/TP ──────────────────────────────────────────

@app.put("/position/{ticket}")
def modify_position(ticket: int, sl: float | None = None, tp: float | None = None):
    ensure_mt5()

    positions = mt5.positions_get(ticket=ticket)
    if not positions:
        raise HTTPException(404, f"Position #{ticket} not found")

    pos = positions[0]
    request = {
        "action": mt5.TRADE_ACTION_SLTP,
        "symbol": pos.symbol,
        "position": ticket,
        "sl": sl if sl is not None else pos.sl,
        "tp": tp if tp is not None else pos.tp,
    }

    result = mt5.order_send(request)
    if result is None:
        raise HTTPException(502, f"Modify failed: {mt5.last_error()}")
    if result.retcode != mt5.TRADE_RETCODE_DONE:
        raise HTTPException(502, f"Modify failed: {result.comment}")

    return {"success": True, "ticket": ticket, "sl": request["sl"], "tp": request["tp"]}

# ── Run ─────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import uvicorn

    parser = argparse.ArgumentParser(description="MT5 Bridge for GreedyClaw")
    parser.add_argument("--port", type=int, default=7879, help="Port (default: 7879)")
    parser.add_argument("--host", default="127.0.0.1", help="Host (default: 127.0.0.1)")
    parser.add_argument("--mt5-path", default=None, help="Path to terminal64.exe")
    args = parser.parse_args()

    MT5_PATH = args.mt5_path

    log.info("Starting MT5 Bridge on %s:%d", args.host, args.port)
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")
