<p align="center">
  <img src="src/clawicon.png" alt="GreedyClaw" width="200"/>
</p>

<h3 align="center">Autonomous AI Trading Agent + Execution Gateway</h3>

<p align="center">
  <strong>AI researches. AI decides. GreedyClaw executes.</strong><br/>
  Self-hosted autonomous trading agent with Rust execution gateway.<br/>
  <strong>6 LLM providers</strong> &bull; <strong>100+ exchanges</strong> &bull; crypto, forex, gold, stocks, DeFi.
</p>

<p align="center">
  <a href="https://github.com/GreedyClaw/GreedyClaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/GreedyClaw/GreedyClaw/ci.yml?style=for-the-badge&label=build" alt="Build"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/releases"><img src="https://img.shields.io/github/v/release/GreedyClaw/GreedyClaw?style=for-the-badge&color=orange" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="License"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/stargazers"><img src="https://img.shields.io/github/stars/GreedyClaw/GreedyClaw?style=for-the-badge&color=yellow" alt="Stars"></a>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> &bull;
  <a href="#ai-brain">AI Brain</a> &bull;
  <a href="#supported-exchanges">Exchanges</a> &bull;
  <a href="#mcp-server">MCP</a> &bull;
  <a href="#risk-engine">Risk Engine</a> &bull;
  <a href="#scanner">Scanner</a> &bull;
  <a href="#docker">Docker</a> &bull;
  <a href="#roadmap">Roadmap</a>
</p>

---

## The Problem

Every AI trading project reinvents the same wheel: exchange authentication, order signing, position tracking, risk limits. Meanwhile, one hallucination loop can drain your entire account in seconds.

**GreedyClaw** is a fully autonomous AI trading system:
- **Brain** (Python) — researches markets, analyzes news, makes decisions using any LLM
- **Gateway** (Rust) — executes trades, enforces risk limits, keeps audit trail

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        GreedyClaw System                                 │
│                                                                          │
│  ┌─────────────────────────────┐       ┌──────────────────────────────┐ │
│  │      Brain (Python)         │       │      Gateway (Rust)          │ │
│  │                             │       │                              │ │
│  │  Web Scraping               │       │  Auth Middleware             │ │
│  │  ├─ Forex Factory           │ POST  │  Risk Engine                 │ │
│  │  ├─ Reuters / Bloomberg     │ /trade│  ├─ Symbol whitelist         │ │
│  │  └─ Any URL                 │──────►│  ├─ Position limits         │ │
│  │                             │       │  ├─ Daily loss cap          │ │
│  │  LLM Analysis (6 providers) │       │  └─ Hallucination detector  │ │
│  │  ├─ Anthropic (Claude)      │       │                              │ │
│  │  ├─ OpenAI (GPT)            │       │  Exchange Layer              │──► Binance
│  │  ├─ Google (Gemini)         │       │  ├─ Binance (native)        │──► MT5 (Forex)
│  │  ├─ DeepSeek                │       │  ├─ PumpFun (native)        │──► Bybit
│  │  ├─ OpenRouter (200+)       │       │  ├─ MT5 (bridge)            │──► OKX
│  │  └─ Ollama (local)          │       │  └─ CCXT (100+ exchanges)   │──► 100+ more
│  │                             │       │                              │ │
│  │  Skills (SKILL.md)          │       │  Audit Log                   │ │
│  │  ├─ forex-fundamentals      │       │  └─ SQLite + JSONL           │ │
│  │  ├─ xauusd-sentiment        │       │                              │ │
│  │  └─ crypto-momentum         │       │  Dashboard (web UI)          │ │
│  └─────────────────────────────┘       └──────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
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

### Docker (recommended)

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw

# Set your keys
cp .env.example .env
nano .env  # Add API keys

# Start everything
docker-compose up
```

### Manual install

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw
cargo build --release
./target/release/greedyclaw init

# Edit your API keys
nano ~/.greedyclaw/.env
nano ~/.greedyclaw/config.toml

# Start the gateway
greedyclaw serve
```

### First Trade

```bash
curl -X POST http://127.0.0.1:7878/trade \
  -H "Authorization: Bearer your_token" \
  -H "Content-Type: application/json" \
  -d '{"action": "buy", "symbol": "BTCUSDT", "amount": 0.001}'
```

## AI Brain

The Brain is an autonomous Python agent that researches markets and trades through the Gateway. Inspired by the [OpenClaw](https://github.com/openclaw/openclaw) agent architecture.

### How it works

```
Every 15 minutes:
  1. OBSERVE  — Check positions, balance, risk status
  2. RESEARCH — Scrape Forex Factory, news sites, search the web
  3. ANALYZE  — Feed everything to LLM for analysis
  4. DECIDE   — LLM outputs: action + confidence + reasoning
  5. LOG      — Record decision to decisions.jsonl
  6. ACT      — If confidence >= 70%, execute trade via Gateway
```

### Setup

```bash
cd brain
pip install -r requirements.txt

# Interactive onboarding wizard
python -m brain --setup
```

The wizard guides you through:
1. LLM provider selection (set at least one API key)
2. Gateway connection
3. Market selection (forex/crypto/solana)
4. Symbol configuration

### Run

```bash
# Single analysis cycle
python -m brain

# Autonomous loop (every 15 minutes)
python -m brain --loop

# Check status
python -m brain --status
```

### 6 LLM Providers with Automatic Failover

| Provider | Env Variable | Models |
|----------|-------------|--------|
| **Anthropic** | `ANTHROPIC_API_KEY` | Claude Sonnet, Opus |
| **OpenAI** | `OPENAI_API_KEY` | GPT-4o, GPT-4 |
| **Google** | `GOOGLE_API_KEY` | Gemini 2.5 Flash/Pro |
| **DeepSeek** | `DEEPSEEK_API_KEY` | DeepSeek Chat |
| **OpenRouter** | `OPENROUTER_API_KEY` | 200+ models |
| **Ollama** | `OLLAMA_URL` | Llama, Mistral, any local model |

Set multiple keys for automatic failover. If Claude is down, Brain switches to GPT, then Gemini, etc.

### Skills (Trading Strategies as Markdown)

Skills are SKILL.md files that teach the LLM *how* to trade. The agent reads them on-demand:

| Skill | File | Strategy |
|-------|------|----------|
| **forex-fundamentals** | `brain/skills/forex-fundamentals/SKILL.md` | Trade around economic calendar events (NFP, CPI, FOMC) |
| **xauusd-sentiment** | `brain/skills/xauusd-sentiment/SKILL.md` | Gold sentiment scoring: Fed + DXY + geopolitics |
| **crypto-momentum** | `brain/skills/crypto-momentum/SKILL.md` | BTC/ETH momentum: ETF flows + Fear/Greed + on-chain |

Create your own: add a folder in `brain/skills/your-strategy/SKILL.md` — the agent picks it up automatically.

### Agent Tools

The LLM can call these tools during reasoning:

| Tool | Description |
|------|-------------|
| `trade` | Execute buy/sell via Gateway |
| `get_price` | Current bid/ask price |
| `get_positions` | Open positions + PnL |
| `get_balance` | Account equity + margin |
| `get_risk_status` | Risk engine state |
| `web_search` | Search the web (DuckDuckGo) |
| `fetch_url` | Extract text from any URL |
| `log_decision` | Record reasoning + confidence |

## MCP Server

GreedyClaw includes a [Model Context Protocol](https://modelcontextprotocol.io/) server — connect Claude Desktop, Cursor, or VS Code directly.

### Setup (Claude Desktop)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "greedyclaw": {
      "command": "npx",
      "args": ["tsx", "path/to/GreedyClaw/integrations/mcp-server/index.ts"],
      "env": {
        "GREEDYCLAW_AUTH_TOKEN": "your_token",
        "GREEDYCLAW_URL": "http://127.0.0.1:7878"
      }
    }
  }
}
```

### MCP Tools (12 total)

| Tool | Description |
|------|-------------|
| `trade` | Execute buy/sell trade |
| `status` | Gateway health + risk state |
| `balance` | Account balances |
| `positions` | Open positions + PnL |
| `price` | Current price for symbol |
| `trades` | Recent trade history |
| `stats` | Aggregated trade statistics |
| `pnl` | PnL time series |
| `cancel` | Cancel open order |
| `scanner_start` | Start PumpFun token scanner |
| `scanner_stop` | Stop scanner |
| `scanner_status` | Scanner metrics + top tokens |

## Using MT5 (Forex, Gold, Indices)

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

## Using CCXT (Bybit, OKX, Kraken, etc.)

```bash
# 1. Start the CCXT bridge
cd mt5-bridge
pip install ccxt fastapi uvicorn
CCXT_API_KEY=... CCXT_SECRET=... python ccxt_bridge.py --exchange bybit

# 2. Set exchange = "bybit" in config.toml, then:
greedyclaw serve
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

## Docker

One command to run the full stack:

```bash
# Copy and edit .env
cp .env.example .env
nano .env

# Start Gateway + Brain
docker-compose up

# With MT5 bridge
docker-compose --profile mt5 up

# With CCXT bridge (Bybit, OKX, etc.)
CCXT_EXCHANGE=bybit docker-compose --profile ccxt up
```

### Services

| Service | Image | Port | Description |
|---------|-------|------|-------------|
| `gateway` | Rust | 7878 | Execution gateway |
| `brain` | Python | — | Autonomous AI agent |
| `mt5-bridge` | Python | 7879 | MT5 connector (optional) |
| `ccxt-bridge` | Python | 7880 | CCXT connector (optional) |

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

### `~/.greedyclaw/brain.yaml`

```yaml
model: claude-sonnet-4-20250514
loop_interval_minutes: 15
market: forex
symbols:
  - XAUUSD
sources:
  - forex_factory
  - investing_com
```

### `~/.greedyclaw/.env`

```env
GREEDYCLAW_AUTH_TOKEN=your_auth_token

# Exchange keys
BINANCE_API_KEY=your_key
BINANCE_SECRET_KEY=your_secret

# LLM providers (set at least one)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
GOOGLE_API_KEY=AI...
# DEEPSEEK_API_KEY=sk-...
# OPENROUTER_API_KEY=sk-or-...
# OLLAMA_URL=http://localhost:11434
```

## Architecture

```
GreedyClaw/
├── src/                         # Rust gateway
│   ├── main.rs                  # CLI: init, serve, trade
│   ├── config.rs                # TOML + .env config loading
│   ├── server.rs                # Axum router, auth middleware
│   ├── dashboard.rs             # Embedded HTML/JS dashboard
│   ├── error.rs                 # LLM-friendly error responses
│   ├── risk.rs                  # Risk engine (mandatory)
│   ├── audit.rs                 # SQLite + JSONL dual-write
│   ├── exchange/
│   │   ├── mod.rs               # Exchange trait (5 methods)
│   │   ├── types.rs             # OrderRequest, OrderResult, Balance
│   │   ├── binance.rs           # Binance REST + HMAC-SHA256
│   │   ├── pumpfun.rs           # PumpFun bonding curve (Solana)
│   │   ├── pumpswap.rs          # PumpSwap AMM (Solana)
│   │   ├── mt5.rs               # MetaTrader 5 (via bridge)
│   │   └── ccxt.rs              # CCXT 100+ exchanges (via bridge)
│   ├── scanner/                 # PumpFun token scanner
│   ├── api/                     # REST API handlers
│   └── solana/                  # Solana wallet, RPC, TX building
│
├── brain/                       # Python autonomous agent
│   ├── main.py                  # Entry point + onboarding wizard
│   ├── agent.py                 # Core loop: observe → think → act
│   ├── llm.py                   # Multi-provider LLM (6 providers)
│   ├── tools.py                 # 8 tools for LLM reasoning
│   ├── scraper.py               # Web scraping (Forex Factory, news)
│   ├── memory.py                # JSONL session + decision persistence
│   ├── config.py                # brain.yaml + env config
│   └── skills/                  # Trading strategies (SKILL.md)
│       ├── forex-fundamentals/  # Economic calendar trading
│       ├── xauusd-sentiment/    # Gold sentiment analysis
│       └── crypto-momentum/     # Crypto momentum trading
│
├── mt5-bridge/                  # Python bridges
│   ├── mt5_bridge.py            # MT5 FastAPI bridge
│   ├── ccxt_bridge.py           # CCXT FastAPI bridge
│   └── requirements.txt
│
├── integrations/
│   ├── mcp-server/              # MCP server (12 tools)
│   └── openclaw-skill/          # OpenClaw skill integration
│
├── Dockerfile                   # Rust gateway image
├── docker-compose.yml           # Full stack: gateway + brain + bridges
├── install.sh                   # macOS/Linux installer
└── install.ps1                  # Windows installer
```

## Roadmap

- [x] **Phase 1: MVP** — Binance Testnet, REST API, risk engine, audit log
- [x] **Phase 2: Solana** — PumpFun + PumpSwap, Ed25519 signing, Jupiter
- [x] **Phase 3: Dashboard** — Visual trading dashboard, PnL charts
- [x] **Phase 4: Scanner** — PumpFun token discovery, LAZARUS strategy, gRPC streaming
- [x] **Phase 5: Multi-exchange** — MT5 (Forex/Gold) + CCXT (100+ exchanges)
- [x] **Phase 8: MCP Server** — Model Context Protocol (12 tools for Claude/Cursor/VS Code)
- [x] **Phase 9: AI Brain** — Autonomous agent, 6 LLM providers, skills, web scraping
- [x] **Docker** — One-command deployment (docker-compose up)
- [ ] **Phase 6: Auto-trade** — Scanner triggers wired to Brain execution
- [ ] **Phase 7: WebSocket** — Real-time feeds and fill notifications
- [ ] **Phase 10: Strategy SDK** — Pluggable strategy modules with backtesting
- [ ] **Phase 11: Telegram Bot** — Mobile notifications and control

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
- **LLM failover** — if one provider goes down, Brain switches automatically

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
  <img src="src/clawicon.png" width="80"/>
  <br/>
  <strong>Built with Rust. Powered by AI. Guarded by risk limits.</strong>
</p>
