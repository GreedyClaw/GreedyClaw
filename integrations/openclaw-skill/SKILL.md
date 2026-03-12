---
name: greedyclaw
description: AI-native trading execution gateway — buy/sell crypto on Binance, PumpFun (Solana bonding curve), and PumpSwap (Solana AMM) via local REST API
homepage: https://github.com/GreedyClaw/GreedyClaw
metadata:
  {
    "openclaw":
      {
        "emoji": "🦀",
        "requires": { "bins": [] },
      },
  }
---

# GreedyClaw — Trading Execution Gateway

You have access to a local trading execution gateway at `http://127.0.0.1:7878`.
All API calls require a Bearer token in the Authorization header.

## Authentication

Every request must include:
```
Authorization: Bearer <GREEDYCLAW_AUTH_TOKEN>
```

The token is stored in `~/.greedyclaw/.env` as `GREEDYCLAW_AUTH_TOKEN`.

## Available Endpoints

### Execute a Trade
```
POST http://127.0.0.1:7878/trade
Content-Type: application/json

{
  "action": "buy" or "sell",
  "symbol": "BTCUSDT" (Binance) or mint address (Solana),
  "amount": 0.001,
  "order_type": "market" (default) or "limit",
  "price": 95000.0  (required for limit orders only)
}
```

Response includes fill details + risk snapshot:
```json
{
  "success": true,
  "order_id": "...",
  "symbol": "BTCUSDT",
  "side": "buy",
  "filled_qty": 0.001,
  "avg_price": 95432.50,
  "status": "Filled",
  "commission": 0.00009,
  "risk": {
    "open_positions": 1,
    "realized_daily_pnl": 0.0,
    "remaining_daily_limit": 100.0,
    "trades_last_minute": 1
  }
}
```

### Check Status
```
GET http://127.0.0.1:7878/status
```
Returns: exchange name, testnet flag, version, risk snapshot.

### Get Balance
```
GET http://127.0.0.1:7878/balance
```
Returns: account balances (USD, BTC, SOL, etc.).

### Get Positions
```
GET http://127.0.0.1:7878/positions
```
Returns: open positions with entry price, current price, unrealized PnL.

### Get Price
```
GET http://127.0.0.1:7878/price/BTCUSDT
```
Returns: current price for a symbol.

### Trade History
```
GET http://127.0.0.1:7878/trades
```
Returns: last 50 trades from audit log.

### Trade Statistics
```
GET http://127.0.0.1:7878/trades/stats
```
Returns: total trades, buys, sells, volume, commissions, win rate.

### Cancel Order
```
DELETE http://127.0.0.1:7878/order/BTCUSDT:12345
```

## Supported Exchanges

The active exchange is set in `~/.greedyclaw/config.toml`:
- **binance** — Binance Spot (testnet or production)
- **pumpfun** — PumpFun bonding curve tokens on Solana
- **pumpswap** — PumpSwap AMM graduated tokens on Solana

For Solana exchanges:
- `symbol` = token mint address (base58)
- Buy `amount` = SOL to spend
- Sell `amount` = raw token count (not SOL)

## Risk Engine

Every trade passes through a risk engine that enforces:
- Max position size (default $500)
- Max daily loss (default $100)
- Max open positions (default 3)
- Rate limit (default 10 trades/min — prevents hallucination loops)
- Symbol whitelist (if configured)

If a trade is rejected, the response explains why:
```json
{
  "success": false,
  "error": "daily loss limit reached",
  "code": "RISK_VIOLATION",
  "suggestion": "Wait until tomorrow or increase max_daily_loss_usd in config"
}
```

## Dashboard

Visual trading dashboard available at: `http://127.0.0.1:7878/dashboard`

## Important Notes

- Always check `/status` first to confirm the gateway is running
- Use `web_fetch` tool to call these endpoints with proper Authorization header
- The gateway runs on localhost only — it is not exposed to the internet
- All trades are logged to SQLite (`~/.greedyclaw/trades.db`) for audit
- When the user asks to "find a hype token and buy it", search for trending tokens first (e.g., via web search), then execute the trade via this API
