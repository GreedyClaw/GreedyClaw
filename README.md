<p align="center">
  <img src="https://img.shields.io/badge/GREEDYCLAW-000000?style=for-the-badge&logo=rust&logoColor=white" alt="GreedyClaw" height="60"/>
</p>

<h3 align="center">AI-Native Trading Execution Gateway</h3>

<p align="center">
  <strong>Your AI agent trades. GreedyClaw executes.</strong><br/>
  Self-hosted Rust gateway that turns any LLM into a trader — safely.<br/>
  One API for <strong>100+ exchanges</strong>: crypto, forex, gold, stocks, DeFi.
</p>

<p align="center">
  <a href="https://github.com/GreedyClaw/GreedyClaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/GreedyClaw/GreedyClaw/ci.yml?style=for-the-badge&label=build" alt="Build"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/releases"><img src="https://img.shields.io/github/v/release/GreedyClaw/GreedyClaw?style=for-the-badge&color=orange" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="License"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/stargazers"><img src="https://img.shields.io/github/stars/GreedyClaw/GreedyClaw?style=for-the-badge&color=yellow" alt="Stars"></a>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> &bull;
  <a href="#supported-exchanges">Exchanges</a> &bull;
  <a href="#api-reference">API</a> &bull;
  <a href="#risk-engine">Risk Engine</a> &bull;
  <a href="#scanner">Scanner</a> &bull;
  <a href="#configuration">Config</a> &bull;
  <a href="#roadmap">Roadmap</a>
</p>

---

## The Problem

Every AI trading project reinvents the same wheel: exchange authentication, order signing, position tracking, risk limits. Meanwhile, one hallucination loop can drain your entire account in seconds.

**GreedyClaw** is the missing layer between your AI agent and the exchange. A local REST API server that handles execution, enforces risk limits, and keeps an audit trail — so your agent can focus on *what* to trade, not *how*.

```
┌─────────────────────┐       POST /trade        ┌──────────────────┐       ┌─────────────┐
│                     │  ───────────────────────► │                  │ ────► │  Binance    │
│   Your AI Agent     │  { "action": "buy",       │   GreedyClaw     │ ────► │  Bybit      │
│                     │    "symbol": "XAUUSD",    │   (localhost)    │ ────► │  MT5 (Forex)│
│  Claude / GPT /     │    "amount": 0.01 }       │                  │ ────► │  OKX        │
│  Local LLM /        │  ◄─────────────────────── │  ► Risk Check    │ ────► │  Kraken     │
│  Python script      │  { "success": true,       │  ► Exchange API  │ ────► │  PumpFun    │
│                     │    "avg_price": 2650 }     │  ► Audit Log     │ ────► │  100+ more  │
└─────────────────────┘                           └──────────────────┘       └─────────────┘
```

## Supported Exchanges

### Native (built-in, zero dependencies)

| Exchange | Markets | Status |
|----------|---------|--------|
| **Binance** | BTC, ETH, 500+ crypto pairs | Ready |
| **PumpFun** | Solana bonding curve memecoins | Ready |
| **PumpSwap** | Solana AMM graduated tokens | Ready |

### MetaTrader 5 (via Python bridge)

| Exchange | Markets | Status |
|----------|---------|--------|
| **MT5** | Forex (EURUSD, GBPUSD...), Gold (XAUUSD), Indices, Stocks, Crypto CFD | Ready |

### CCXT (via Python bridge — 100+ exchanges)

| Exchange | Type | | Exchange | Type |
|----------|------|-|----------|------|
| **Bybit** | Spot + Futures | | **Gate.io** | Spot + Futures |
| **OKX** | Spot + Futures | | **KuCoin** | Spot + Futures |
| **Kraken** | Spot + Margin | | **Bitget** | Spot + Futures |
| **Coinbase** | Spot | | **MEXC** | Spot + Futures |
| **HTX** | Spot + Futures | | **+ 90 more** | [Full list](https://github.com/ccxt/ccxt/wiki/Exchange-Markets) |

> **One API to rule them all.** Your AI agent calls `POST /trade` — GreedyClaw routes to any exchange.

## Quickstart

### One-line install

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.ps1 | iex
```

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.sh | bash
```

### Manual install

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw
cargo build --release
./target/release/greedyclaw init
```

### Setup

```bash
# Edit your API keys
nano ~/.greedyclaw/.env

# Choose your exchange in config
nano ~/.greedyclaw/config.toml
```

### Run

```bash
# Start the gateway
greedyclaw serve

# 🦀 GreedyClaw v0.1.0 listening on 127.0.0.1:7878
#    GET  /dashboard — visual trading dashboard
#    POST /trade     — execute trades
#    GET  /status    — health + risk snapshot
```

### First Trade

```bash
curl -X POST http://127.0.0.1:7878/trade \
  -H "Authorization: Bearer your_token" \
  -H "Content-Type: application/json" \
  -d '{"action": "buy", "symbol": "BTCUSDT", "amount": 0.001}'
```

### Using MT5 (Forex, Gold, Indices)

```bash
# 1. Start the MT5 bridge (requires MT5 terminal + Python)
cd mt5-bridge
pip install -r requirements.txt
python mt5_bridge.py

# 2. Set exchange = "mt5" in config.toml, then:
greedyclaw serve

# 3. Trade gold!
greedyclaw trade buy XAUUSD 0.01
```

### Using CCXT (Bybit, OKX, Kraken, etc.)

```bash
# 1. Start the CCXT bridge
cd mt5-bridge
pip install ccxt fastapi uvicorn
CCXT_API_KEY=... CCXT_SECRET=... python ccxt_bridge.py --exchange bybit

# 2. Set exchange = "bybit" in config.toml, then:
greedyclaw serve
```

## How It Works

```
                        ┌─────────────────────────────────────────┐
                        │            GreedyClaw Gateway            │
                        │                                         │
  AI Agent ──POST──►    │  ┌───────────┐   ┌──────────────────┐  │
                        │  │  Auth     │──►│  Risk Engine      │  │
                        │  │  Middleware│   │  • Symbol filter  │  │
                        │  └───────────┘   │  • Position limits│  │
                        │                  │  • Daily loss cap  │  │
                        │                  │  • Rate limiter    │  │
                        │                  └────────┬───────────┘  │
                        │                           │ OK           │
                        │                  ┌────────▼───────────┐  │
                        │                  │  Exchange Layer     │  │
                        │                  │  (trait-based)      │  │
                        │                  │                     │  │
                        │                  │  ► Binance (native) │  │
                        │                  │  ► PumpFun (native) │  │
                        │                  │  ► MT5 (bridge)     │  │
                        │                  │  ► CCXT (bridge)    │  │
                        │                  └────────┬───────────┘  │
                        │                           │ Fill         │
                        │                  ┌────────▼───────────┐  │
                        │                  │  Audit Log          │  │
                        │                  │  SQLite + JSONL     │  │
                        │                  └────────────────────┘  │
                        └─────────────────────────────────────────┘
```

## API Reference

All endpoints require `Authorization: Bearer <token>` header (except `/dashboard`).

### Trading

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/trade` | Execute a trade (buy/sell, market/limit) |
| `DELETE` | `/order/{id}` | Cancel an open order |

### Account

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/status` | Health check + risk state |
| `GET` | `/balance` | Account balances |
| `GET` | `/positions` | Open positions + unrealized PnL |
| `GET` | `/price/{symbol}` | Current price |
| `GET` | `/trades` | Recent trades from audit log |
| `GET` | `/trades/stats` | Trade statistics |
| `GET` | `/trades/pnl` | PnL time series |

### Scanner (PumpFun Token Discovery)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/scanner/start` | Start gRPC token scanner |
| `POST` | `/scanner/stop` | Stop scanner |
| `GET` | `/scanner/status` | Scanner metrics + top tokens |
| `GET` | `/scanner/tokens` | All tracked tokens |
| `GET/PUT` | `/scanner/config` | Get/update scanner config |
| `GET` | `/scanner/positions` | Scanner-managed positions |

### Dashboard

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/dashboard` | Visual trading dashboard (no auth) |

## Risk Engine

The risk engine is **mandatory and cannot be disabled**. This is by design — an AI agent with unrestricted exchange access is a liability.

| Protection | Default | Purpose |
|------------|---------|---------|
| **Symbol whitelist** | configurable | Prevent trading unknown pairs |
| **Max position size** | $500 | Cap single trade exposure |
| **Max daily loss** | $100 | Kill switch — stops all trading |
| **Max open positions** | 3 | Prevent over-diversification |
| **Rate limiter** | 10/min | Circuit breaker for hallucination loops |

### Why a Circuit Breaker?

LLMs sometimes enter infinite loops when they receive unexpected errors. GreedyClaw's rate limiter detects this pattern and returns `429 RATE_LIMIT` with *"Possible hallucination loop"* — giving the agent (and you) time to recover.

## Scanner

GreedyClaw includes a built-in **PumpFun token scanner** that streams Solana transactions via Yellowstone gRPC, scores tokens using the LAZARUS strategy (Optuna-optimized), and can autonomously trade:

- Real-time bonding curve tracking
- Anti-rug filters (whale detection, sell ratio, zombie tokens)
- Configurable trigger parameters via API
- Visual dashboard with live token metrics

## Configuration

### `~/.greedyclaw/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 7878

[exchange]
# Native: "binance", "pumpfun", "pumpswap", "mt5"
# CCXT: "bybit", "okx", "kraken", "coinbase", "kucoin", ...
name = "binance"
testnet = true

[risk]
max_position_usd = 500.0
max_daily_loss_usd = 100.0
max_open_positions = 3
allowed_symbols = ["BTCUSDT", "ETHUSDT"]
max_trades_per_minute = 10
```

### `~/.greedyclaw/.env`

```env
GREEDYCLAW_AUTH_TOKEN=your_auth_token

# Binance
BINANCE_API_KEY=your_key
BINANCE_SECRET_KEY=your_secret

# MT5 bridge
# MT5_BRIDGE_URL=http://127.0.0.1:7879

# CCXT bridge (Bybit, OKX, etc.)
# CCXT_BRIDGE_URL=http://127.0.0.1:7880
# CCXT_API_KEY=your_key
# CCXT_SECRET=your_secret
```

## Architecture

```
src/
├── main.rs              # CLI: init, serve, trade
├── config.rs            # TOML + .env config loading
├── server.rs            # Axum router, auth middleware
├── dashboard.rs         # Embedded HTML/JS dashboard
├── error.rs             # LLM-friendly error responses
├── risk.rs              # Risk engine (mandatory)
├── audit.rs             # SQLite + JSONL dual-write
├── exchange/
│   ├── mod.rs           # Exchange trait (5 methods)
│   ├── types.rs         # OrderRequest, OrderResult, Balance
│   ├── binance.rs       # Binance REST + HMAC-SHA256
│   ├── pumpfun.rs       # PumpFun bonding curve (Solana)
│   ├── pumpswap.rs      # PumpSwap AMM (Solana)
│   ├── mt5.rs           # MetaTrader 5 (via bridge)
│   └── ccxt.rs          # CCXT 100+ exchanges (via bridge)
├── scanner/
│   ├── mod.rs           # Scanner service
│   ├── parser.rs        # PumpFun event parser
│   ├── aggregator.rs    # In-memory token tracking
│   ├── scoring.rs       # LAZARUS trigger strategy
│   ├── strategy.rs      # Entry/exit logic
│   └── stream.rs        # gRPC streaming
├── api/
│   ├── mod.rs           # AppState, route registration
│   ├── trade.rs         # POST /trade handler
│   ├── status.rs        # GET endpoints
│   ├── scanner_api.rs   # Scanner API handlers
│   └── types.rs         # Request/response DTOs
└── solana/              # Solana wallet, RPC, TX building

mt5-bridge/
├── mt5_bridge.py        # MT5 Python bridge (FastAPI)
├── ccxt_bridge.py       # CCXT Python bridge (FastAPI)
└── requirements.txt     # Python dependencies
```

## Roadmap

- [x] **Phase 1: MVP** — Binance Testnet, REST API, risk engine, audit log
- [x] **Phase 2: Solana** — PumpFun + PumpSwap, Ed25519 signing, Jupiter
- [x] **Phase 3: Dashboard** — Visual trading dashboard, PnL charts
- [x] **Phase 4: Scanner** — PumpFun token discovery, LAZARUS strategy, gRPC streaming
- [x] **Phase 5: Multi-exchange** — MT5 (Forex/Gold) + CCXT (100+ exchanges)
- [ ] **Phase 6: Auto-trade** — Scanner triggers → real trade execution
- [ ] **Phase 7: WebSocket** — Real-time feeds and fill notifications
- [ ] **Phase 8: MCP Server** — Model Context Protocol for Claude/GPT native integration
- [ ] **Phase 9: Strategy SDK** — Pluggable strategy modules with backtesting

## Use with AI Agents

### Python

```python
import requests

GW = "http://127.0.0.1:7878"
H = {"Authorization": "Bearer your_token", "Content-Type": "application/json"}

# Buy gold on MT5
requests.post(f"{GW}/trade", headers=H, json={"action": "buy", "symbol": "XAUUSD", "amount": 0.01})

# Buy BTC on Binance
requests.post(f"{GW}/trade", headers=H, json={"action": "buy", "symbol": "BTCUSDT", "amount": 0.001})

# Check positions
requests.get(f"{GW}/positions", headers=H).json()
```

### Claude / GPT (Function Calling)

```json
{
  "name": "execute_trade",
  "description": "Execute a trade via GreedyClaw. Supports 100+ exchanges.",
  "parameters": {
    "type": "object",
    "properties": {
      "action": {"type": "string", "enum": ["buy", "sell"]},
      "symbol": {"type": "string", "description": "Trading pair (BTCUSDT, XAUUSD, EURUSD, etc.)"},
      "amount": {"type": "number", "description": "Quantity"}
    },
    "required": ["action", "symbol", "amount"]
  }
}
```

## Security

- **Keys stay local** — runs on `127.0.0.1` only
- **Bearer token auth** — every request authenticated
- **Mandatory risk limits** — cannot be disabled
- **Audit trail** — SQLite + JSONL with fsync
- **No telemetry** — zero data collection, fully offline

## Contributing

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw
cargo build
cargo test
```

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with Rust. Guarded by risk limits. Powered by greed.</strong>
</p>
