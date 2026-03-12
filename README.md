<p align="center">
  <img src="https://img.shields.io/badge/GREEDYCLAW-000000?style=for-the-badge&logo=rust&logoColor=white" alt="GreedyClaw" height="60"/>
</p>

<h3 align="center">AI-Native Trading Execution Gateway</h3>

<p align="center">
  <strong>Your AI agent trades. GreedyClaw executes.</strong><br/>
  Self-hosted Rust gateway that turns any LLM into a trader — safely.
</p>

<p align="center">
  <a href="https://github.com/GreedyClaw/GreedyClaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/GreedyClaw/GreedyClaw/ci.yml?style=for-the-badge&label=build" alt="Build"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/releases"><img src="https://img.shields.io/github/v/release/GreedyClaw/GreedyClaw?style=for-the-badge&color=orange" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="License"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/stargazers"><img src="https://img.shields.io/github/stars/GreedyClaw/GreedyClaw?style=for-the-badge&color=yellow" alt="Stars"></a>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> &bull;
  <a href="#how-it-works">How It Works</a> &bull;
  <a href="#api-reference">API</a> &bull;
  <a href="#risk-engine">Risk Engine</a> &bull;
  <a href="#configuration">Config</a> &bull;
  <a href="#roadmap">Roadmap</a>
</p>

---

## The Problem

Every AI trading project reinvents the same wheel: exchange authentication, order signing, position tracking, risk limits. Meanwhile, one hallucination loop can drain your entire account in seconds.

**GreedyClaw** is the missing layer between your AI agent and the exchange. A local REST API server that handles execution, enforces risk limits, and keeps an audit trail — so your agent can focus on *what* to trade, not *how*.

```
┌─────────────────────┐       POST /trade        ┌──────────────────┐
│                     │  ───────────────────────► │                  │
│   Your AI Agent     │  { "action": "buy",       │   GreedyClaw     │
│                     │    "symbol": "BTCUSDT",   │   (localhost)    │
│  Claude / GPT /     │    "amount": 0.001 }      │                  │
│  Local LLM /        │  ◄─────────────────────── │  ► Risk Check    │
│  Python script      │  { "success": true,       │  ► Exchange API  │
│                     │    "filled_qty": 0.001,   │  ► Audit Log     │
│                     │    "avg_price": 95432 }   │  ► Position Track │
└─────────────────────┘                           └──────────────────┘
```

## Why GreedyClaw?

| Feature | CCXT | Freqtrade | Alpaca | **GreedyClaw** |
|---------|------|-----------|--------|----------------|
| Purpose-built for AI agents | No | No | Partial | **Yes** |
| Self-hosted (keys never leave) | N/A | Yes | No | **Yes** |
| Built-in risk limits | No | Basic | No | **Yes** |
| Circuit breaker for LLM loops | No | No | No | **Yes** |
| REST API for any language | No | No | Yes | **Yes** |
| Rust performance | No | No | N/A | **Yes** |
| Audit trail (SQLite + JSONL) | No | No | Yes | **Yes** |

## Quickstart

### Install

```bash
# From source (requires Rust 1.75+)
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw
cargo build --release

# Binary is at target/release/greedyclaw
```

### Setup

```bash
# Create config directory with defaults
greedyclaw init

# Edit your API keys
# (get testnet keys at https://testnet.binance.vision/)
nano ~/.greedyclaw/.env
```

```env
BINANCE_API_KEY=your_testnet_api_key
BINANCE_SECRET_KEY=your_testnet_secret_key
GREEDYCLAW_AUTH_TOKEN=your_random_token_here
```

### Run

```bash
# Start the gateway
greedyclaw serve

# 🦀 GreedyClaw v0.1.0 listening on 127.0.0.1:7878
#    POST /trade    — execute trades
#    GET  /status   — health + risk snapshot
#    GET  /balance  — account balances
#    GET  /positions — open positions
#    GET  /trades   — audit log
```

### First Trade

```bash
# From your AI agent, Python script, or just curl:
curl -X POST http://127.0.0.1:7878/trade \
  -H "Authorization: Bearer your_random_token_here" \
  -H "Content-Type: application/json" \
  -d '{"action": "buy", "symbol": "BTCUSDT", "amount": 0.001}'
```

```json
{
  "success": true,
  "order_id": "12345678",
  "symbol": "BTCUSDT",
  "side": "buy",
  "filled_qty": 0.001,
  "avg_price": 95432.50,
  "status": "Filled",
  "commission": 0.00009543,
  "timestamp": "2026-03-12T10:30:00Z",
  "risk": {
    "open_positions": 1,
    "max_open_positions": 3,
    "realized_daily_pnl": 0.0,
    "remaining_daily_limit": 100.0,
    "trades_last_minute": 1,
    "max_trades_per_minute": 10
  }
}
```

### CLI Trading

```bash
# Trade directly from the command line (no REST needed)
greedyclaw trade buy BTCUSDT 0.001
greedyclaw trade sell BTCUSDT 0.001
```

## How It Works

```
                        ┌─────────────────────────────────────────┐
                        │            GreedyClaw Gateway            │
                        │                                         │
  AI Agent ──POST──►    │  ┌───────────┐   ┌──────────────────┐  │
                        │  │  Auth     │──►│  Risk Engine      │  │
                        │  │  Middleware│   │                  │  │
                        │  └───────────┘   │  • Symbol whitelist│  │
                        │                  │  • Position limits │  │
                        │                  │  • Daily loss cap  │  │
                        │                  │  • Rate limiter    │  │
                        │                  │    (circuit breaker│  │
                        │                  │     for LLM loops) │  │
                        │                  └────────┬───────────┘  │
                        │                           │ OK           │
                        │                  ┌────────▼───────────┐  │
                        │                  │  Exchange Layer     │  │
                        │                  │  (trait-based)      │  │
                        │                  │                     │  │
                        │                  │  ► Binance (REST)   │  │
                        │                  │  ► Solana (planned) │  │
                        │                  │  ► More exchanges   │  │
                        │                  └────────┬───────────┘  │
                        │                           │ Fill         │
                        │                  ┌────────▼───────────┐  │
                        │                  │  Audit Log          │  │
                        │                  │  SQLite + JSONL     │  │
                        │                  │  (crash-safe)       │  │
                        │                  └────────────────────┘  │
                        └─────────────────────────────────────────┘
```

**Key design decisions:**
- **Local-first** — your API keys never leave your machine
- **Trait-based exchange abstraction** — adding a new exchange = implementing 5 methods
- **Mandatory risk engine** — cannot be disabled, sane defaults out of the box
- **Dual audit log** — SQLite for queries + JSONL with fsync for crash recovery
- **LLM-friendly errors** — every error response includes a `suggestion` field

## API Reference

All endpoints require `Authorization: Bearer <token>` header.

### `POST /trade`

Execute a trade.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `action` | string | Yes | `"buy"` or `"sell"` |
| `symbol` | string | Yes | Trading pair, e.g. `"BTCUSDT"` |
| `amount` | number | Yes | Quantity in base asset |
| `order_type` | string | No | `"market"` (default) or `"limit"` |
| `price` | number | No | Required for limit orders |

### `GET /status`

Health check + current risk state.

### `GET /balance`

Account balances from the exchange.

### `GET /positions`

Open positions with current prices and unrealized PnL.

### `GET /price/{symbol}`

Current price for a symbol.

### `GET /trades`

Recent trades from the audit log (last 50).

### `DELETE /order/{symbol}:{orderId}`

Cancel an open order.

### Error Responses

Every error includes machine-readable `code` and human-readable `suggestion`:

```json
{
  "success": false,
  "error": "Risk limit exceeded: daily loss limit $100 reached",
  "code": "RISK_VIOLATION",
  "suggestion": "Risk limit hit. Check GET /status for current limits and usage."
}
```

| Code | HTTP | Meaning |
|------|------|---------|
| `RISK_VIOLATION` | 403 | Trade blocked by risk engine |
| `RATE_LIMIT` | 429 | Too many trades/minute (possible LLM loop) |
| `VALIDATION_ERROR` | 400 | Bad request parameters |
| `EXCHANGE_ERROR` | 502 | Exchange rejected the order |
| `EXCHANGE_UNREACHABLE` | 502 | Cannot reach exchange API |

## Risk Engine

The risk engine is **mandatory and cannot be disabled**. This is by design — an AI agent with unrestricted exchange access is a liability.

### Protections

| Protection | Default | Purpose |
|------------|---------|---------|
| **Symbol whitelist** | `["BTCUSDT", "ETHUSDT"]` | Prevent trading unknown pairs |
| **Max position size** | $500 | Cap single trade exposure |
| **Max daily loss** | $100 | Kill switch — stops all trading |
| **Max open positions** | 3 | Prevent over-diversification |
| **Rate limiter** | 10/min | Circuit breaker for hallucination loops |

### Why a Circuit Breaker?

LLMs sometimes enter infinite loops when they receive unexpected errors. Without a rate limit, a buggy agent can:
1. Place an order → get error
2. Retry immediately → get error
3. Repeat 1000x → drain account on fees

GreedyClaw's rate limiter detects this pattern and returns `429 RATE_LIMIT` with the message *"Possible hallucination loop"* — giving the agent (and you) time to recover.

## Configuration

### `~/.greedyclaw/config.toml`

```toml
[server]
host = "127.0.0.1"    # Loopback only — never expose to network
port = 7878

[exchange]
name = "binance"
testnet = true          # Start with testnet, always

[risk]
max_position_usd = 500.0
max_daily_loss_usd = 100.0
max_open_positions = 3
allowed_symbols = ["BTCUSDT", "ETHUSDT"]
max_trades_per_minute = 10

[logging]
level = "info"          # trace, debug, info, warn, error
format = "pretty"
```

### `~/.greedyclaw/.env`

```env
BINANCE_API_KEY=your_key
BINANCE_SECRET_KEY=your_secret
GREEDYCLAW_AUTH_TOKEN=your_auth_token
```

## Architecture

```
src/
├── main.rs              # CLI: init, serve, trade
├── config.rs            # TOML + .env config loading
├── server.rs            # Axum router, auth middleware
├── error.rs             # LLM-friendly error responses
├── risk.rs              # Risk engine (mandatory)
├── audit.rs             # SQLite + JSONL dual-write
├── exchange/
│   ├── mod.rs           # Exchange trait (5 methods)
│   ├── types.rs         # OrderRequest, OrderResult, Balance
│   └── binance.rs       # Binance REST + HMAC-SHA256
└── api/
    ├── mod.rs           # AppState, route registration
    ├── trade.rs         # POST /trade handler
    ├── status.rs        # GET endpoints
    └── types.rs         # Request/response DTOs
```

### Tech Stack

| Component | Choice | Why |
|-----------|--------|-----|
| Language | **Rust** | Sub-ms latency, memory safety, no GC pauses |
| Async | **Tokio** | Industry standard async runtime |
| HTTP Server | **Axum** | Fast, ergonomic, tower middleware |
| HTTP Client | **Reqwest** | rustls-tls, no OpenSSL dependency |
| Database | **rusqlite** | Bundled SQLite, zero system deps |
| Config | **TOML + dotenvy** | Human-readable, secrets separated |
| Signing | **HMAC-SHA256** | Binance API standard |
| Concurrency | **DashMap** | Lock-free concurrent position tracking |

## Adding a New Exchange

GreedyClaw uses a trait-based exchange abstraction. To add a new exchange, implement 5 methods:

```rust
impl Exchange for MyExchange {
    fn name(&self) -> &str { "my-exchange" }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError>;
    async fn limit_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError>;
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), AppError>;
    async fn get_balance(&self) -> Result<Balance, AppError>;
    async fn get_price(&self, symbol: &str) -> Result<f64, AppError>;
}
```

The AI agent doesn't need to change — it just calls `POST /trade` and GreedyClaw routes to the right exchange.

## Roadmap

- [x] **Phase 1: MVP** — Binance Testnet, REST API, risk engine, audit log
- [ ] **Phase 2: Solana/PumpFun** — Jupiter swaps, Jito tips, gRPC streaming
- [ ] **Phase 3: Multi-exchange** — Run multiple exchanges simultaneously
- [ ] **Phase 4: WebSocket** — Real-time price feeds and fill notifications
- [ ] **Phase 5: MCP Server** — Model Context Protocol for Claude/GPT native integration
- [ ] **Phase 6: Strategy SDK** — Pluggable strategy modules with backtesting

## Security

- **Keys stay local** — GreedyClaw runs on `127.0.0.1` only. Your API keys and auth tokens never leave your machine.
- **Bearer token auth** — Every request requires authentication. No anonymous access.
- **Risk limits** — Even with valid auth, the risk engine prevents catastrophic losses.
- **Audit trail** — Every trade (success or failure) is logged to SQLite + JSONL with fsync.
- **No telemetry** — Zero data collection. No phone-home. Fully offline capable.

## Use with AI Agents

### Python

```python
import requests

GATEWAY = "http://127.0.0.1:7878"
TOKEN = "your_auth_token"
HEADERS = {"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"}

# Buy
r = requests.post(f"{GATEWAY}/trade", headers=HEADERS, json={
    "action": "buy", "symbol": "BTCUSDT", "amount": 0.001
})
print(r.json())

# Check positions
r = requests.get(f"{GATEWAY}/positions", headers=HEADERS)
print(r.json())
```

### Claude / GPT (Function Calling)

GreedyClaw's API is designed for LLM function calling. Define a tool:

```json
{
  "name": "execute_trade",
  "description": "Execute a trade via GreedyClaw gateway",
  "parameters": {
    "type": "object",
    "properties": {
      "action": {"type": "string", "enum": ["buy", "sell"]},
      "symbol": {"type": "string", "description": "Trading pair e.g. BTCUSDT"},
      "amount": {"type": "number", "description": "Quantity in base asset"}
    },
    "required": ["action", "symbol", "amount"]
  }
}
```

### curl

```bash
# Status
curl http://127.0.0.1:7878/status -H "Authorization: Bearer $TOKEN"

# Trade
curl -X POST http://127.0.0.1:7878/trade \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"action": "buy", "symbol": "ETHUSDT", "amount": 0.01}'

# Positions
curl http://127.0.0.1:7878/positions -H "Authorization: Bearer $TOKEN"
```

## Contributing

Contributions welcome! GreedyClaw is in early development — there's a lot to build.

```bash
# Development
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
