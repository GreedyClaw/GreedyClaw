<p align="center">
  <img src="src/clawicon.png" alt="GreedyClaw" width="180"/>
</p>

<h1 align="center">GreedyClaw</h1>

<h3 align="center">Autonomous AI Trading Agent + Hardened Execution Gateway</h3>

<p align="center">
  <strong>AI researches markets. AI makes decisions. Rust executes trades.</strong><br/>
  Self-hosted autonomous trading system with 6-layer security architecture.<br/>
</p>

<p align="center">
  <a href="https://github.com/GreedyClaw/GreedyClaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/GreedyClaw/GreedyClaw/ci.yml?style=for-the-badge&label=CI" alt="CI"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/releases"><img src="https://img.shields.io/github/v/release/GreedyClaw/GreedyClaw?style=for-the-badge&color=orange" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="License"></a>
  <a href="https://github.com/GreedyClaw/GreedyClaw/stargazers"><img src="https://img.shields.io/github/stars/GreedyClaw/GreedyClaw?style=for-the-badge&color=yellow" alt="Stars"></a>
</p>

<p align="center">
  <a href="#why-greedyclaw">Why</a> &bull;
  <a href="#quickstart">Quickstart</a> &bull;
  <a href="#architecture">Architecture</a> &bull;
  <a href="#ai-brain">AI Brain</a> &bull;
  <a href="#supported-exchanges">Exchanges</a> &bull;
  <a href="#security">Security</a> &bull;
  <a href="#risk-engine">Risk Engine</a> &bull;
  <a href="#mcp-server">MCP</a> &bull;
  <a href="#scanner">Scanner</a> &bull;
  <a href="#docker">Docker</a> &bull;
  <a href="#api">API</a> &bull;
  <a href="#roadmap">Roadmap</a>
</p>

---

## Why GreedyClaw

Every AI trading project reinvents the same wheel: exchange auth, order signing, position tracking, risk limits. Meanwhile, one hallucination loop drains your account in seconds. Most AI agent frameworks? **[15,200 instances vulnerable to remote code execution](https://www.oligo.security/blog/ai-in-development-the-emerging-risks-of-mcp).**

GreedyClaw exists because **AI + money requires defense-in-depth, not "hope nothing goes wrong":**

| Component | Language | Purpose |
|-----------|----------|---------|
| **Brain** | Python | Researches markets, scrapes news, reasons with any LLM, makes trading decisions |
| **Gateway** | Rust | Executes trades, enforces risk limits via mandatory engine, logs every action with HMAC-signed audit trail |
| **Isolation** | Architecture | Brain has zero direct access to exchanges, filesystem, or shell. All trades pass through Gateway's risk checks. Even if the LLM is fully compromised by prompt injection, the worst case is a trade that passes risk limits — not a system takeover |

```
            ┌──────────────────────────────────────────────────────────────────┐
            │                         GreedyClaw                               │
            │                                                                  │
 Forex      │  ┌──── Brain (Python) ─────┐     ┌──── Gateway (Rust) ──────┐  │
 Factory ◄──┤  │                         │     │                          │  ├──► Binance
 Reuters ◄──┤  │  Web Scraping           │     │  Constant-time Auth      │  ├──► MT5 (Forex/Gold)
 Any URL ◄──┤  │  6 LLM Providers        │     │  Risk Engine (mandatory) │  ├──► Bybit
            │  │  Trading Skills (.md)    │     │  HMAC-SHA256 Audit Log   │  ├──► OKX
            │  │  Decision Logger         │POST │  1MB Body Limit          │  ├──► Kraken
 Claude  ◄──┤  │                         │────►│  NaN/Inf Rejection       │  ├──► PumpFun (Solana)
 GPT-4o  ◄──┤  │  SSRF Protection        │/trade  Error Sanitization      │  ├──► PumpSwap (Solana)
 Gemini  ◄──┤  │  Confidence Gate (≥70%) │     │  Rate Limiter (429)      │  ├──► 100+ via CCXT
            │  └─────────────────────────┘     └──────────────────────────┘  │
            └──────────────────────────────────────────────────────────────────┘
```

## Quickstart

### Docker (recommended)

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git && cd GreedyClaw
cp .env.example .env && nano .env   # Set API keys
docker-compose up                    # Gateway + Brain running
```

### One-line install

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.ps1 | iex

# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.sh | bash
```

### Manual

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git && cd GreedyClaw
cargo build --release
./target/release/greedyclaw init      # Creates ~/.greedyclaw/ with crypto-random auth token
nano ~/.greedyclaw/.env               # Add exchange + LLM API keys
greedyclaw serve                      # Gateway on 127.0.0.1:7878
```

### First trade

```bash
# From any language, any framework, any AI agent:
curl -X POST http://127.0.0.1:7878/trade \
  -H "Authorization: Bearer your_token" \
  -H "Content-Type: application/json" \
  -d '{"action": "buy", "symbol": "BTCUSDT", "amount": 0.001}'
```

---

## Architecture

```
GreedyClaw/
├── src/                            # Rust Gateway (compiled binary)
│   ├── main.rs                     #   CLI: init, serve, trade
│   ├── server.rs                   #   Axum router + constant-time auth middleware
│   ├── risk.rs                     #   Mandatory risk engine (cannot be disabled)
│   ├── audit.rs                    #   SQLite WAL + JSONL dual-write + HMAC-SHA256
│   ├── error.rs                    #   LLM-friendly errors (sanitized, no internals leaked)
│   ├── config.rs                   #   TOML + .env config
│   ├── dashboard.rs                #   Embedded HTML/JS trading dashboard
│   ├── exchange/
│   │   ├── mod.rs                  #   Exchange trait (5 async methods)
│   │   ├── binance.rs              #   Binance REST + HMAC-SHA256
│   │   ├── pumpfun.rs              #   PumpFun bonding curve (Solana)
│   │   ├── pumpswap.rs             #   PumpSwap AMM (Solana)
│   │   ├── mt5.rs                  #   MetaTrader 5 (via Python bridge)
│   │   └── ccxt.rs                 #   CCXT 100+ exchanges (via Python bridge)
│   ├── scanner/                    #   PumpFun gRPC token scanner
│   ├── api/                        #   REST API handlers
│   └── solana/                     #   Ed25519 wallet, RPC, TX building
│
├── brain/                          # Python AI Agent
│   ├── agent.py                    #   Core loop: observe → think → act
│   ├── llm.py                      #   Multi-provider LLM (6 providers, auto-failover)
│   ├── tools.py                    #   8 tools for LLM reasoning + SSRF protection
│   ├── scraper.py                  #   Web scraping (Forex Factory, Investing.com)
│   ├── memory.py                   #   Session + decision persistence (JSONL)
│   └── skills/                     #   Trading strategies as markdown
│       ├── forex-fundamentals/     #     Economic calendar trading
│       ├── xauusd-sentiment/       #     Gold sentiment analysis
│       └── crypto-momentum/        #     Crypto momentum trading
│
├── mt5-bridge/                     # Python bridges
│   ├── mt5_bridge.py               #   MT5 FastAPI bridge (:7879)
│   └── ccxt_bridge.py              #   CCXT FastAPI bridge (:7880)
│
├── integrations/mcp-server/        # MCP server (12 tools for Claude/Cursor/VS Code)
├── Dockerfile + docker-compose.yml # Full stack deployment
├── install.sh / install.ps1        # One-line installers
└── WIKI/                           # Documentation (25 pages)
```

---

## AI Brain

The Brain is a fully autonomous Python agent. It researches markets, analyzes news, makes decisions, and executes trades — all without human intervention.

### Decision loop

```
Every 15 minutes (configurable):
  1. OBSERVE  — GET /positions, /balance, /status → current state
  2. RESEARCH — Scrape Forex Factory, Reuters, Investing.com, web search
  3. ANALYZE  — Feed everything to LLM with trading skill context
  4. DECIDE   — LLM outputs: action + confidence (0-100) + reasoning
  5. LOG      — Record decision to decisions.jsonl (always, even for "hold")
  6. ACT      — If confidence ≥ 70%, execute trade via Gateway
                 If confidence < 70%, enforced in CODE (not just prompt)
```

### Security gates

The Brain enforces safety at the **code level**, not just via prompt instructions:

- **Confidence threshold** — `agent.py` blocks `POST /trade` calls unless `log_decision` was called with `confidence >= 70`. Even if the LLM ignores the prompt rule, the code blocks the trade.
- **SSRF protection** — `tools.py` blocks `fetch_url` calls to `127.x`, `10.x`, `192.168.x`, `169.254.x`, metadata endpoints. DNS resolution is checked against private IP ranges.
- **API key hygiene** — Google API key sent via `x-goog-api-key` header, never embedded in URLs (prevents leakage via proxy logs).

### 6 LLM providers with automatic failover

| Priority | Provider | Env Variable | Default Model |
|----------|----------|-------------|---------------|
| 0 | **Anthropic** | `ANTHROPIC_API_KEY` | Claude Sonnet 4 |
| 1 | **OpenAI** | `OPENAI_API_KEY` | GPT-4o |
| 2 | **Google** | `GOOGLE_API_KEY` | Gemini 2.5 Flash |
| 3 | **DeepSeek** | `DEEPSEEK_API_KEY` | DeepSeek Chat |
| 5 | **OpenRouter** | `OPENROUTER_API_KEY` | 200+ models |
| 10 | **Ollama** | `OLLAMA_URL` | Llama 3.1 70B (local) |

Set multiple keys — if Claude is down, Brain auto-fails over to GPT, then Gemini, etc.

### Skills (trading strategies as markdown)

Skills are `SKILL.md` files that teach the LLM *how* to trade specific markets:

| Skill | Strategy |
|-------|----------|
| `forex-fundamentals` | Trade around economic events (NFP, CPI, FOMC, rate decisions) |
| `xauusd-sentiment` | Gold scoring: Fed policy + DXY + geopolitics + ETF flows |
| `crypto-momentum` | BTC/ETH momentum: ETF flows + Fear/Greed + on-chain metrics |

Create your own: `brain/skills/your-strategy/SKILL.md` — the agent loads it automatically.

### Agent tools

| Tool | Description |
|------|-------------|
| `trade` | Execute buy/sell via Gateway (blocked if confidence < 70) |
| `get_price` | Current bid/ask |
| `get_positions` | Open positions + unrealized PnL |
| `get_balance` | Account equity + margin |
| `get_risk_status` | Risk engine snapshot |
| `web_search` | DuckDuckGo search (no API key) |
| `fetch_url` | Extract text from URL (SSRF-protected) |
| `log_decision` | Record reasoning + confidence (required before trading) |

### Setup & run

```bash
cd brain && pip install -r requirements.txt
python -m brain --setup            # Interactive onboarding wizard
python -m brain --loop             # Autonomous mode (every 15 min)
```

---

## Supported Exchanges

### Native (zero dependencies, built into Rust binary)

| Exchange | Markets | Protocol |
|----------|---------|----------|
| **Binance** | 500+ crypto pairs | REST + HMAC-SHA256 |
| **PumpFun** | Solana bonding curve memecoins | RPC + Ed25519 |
| **PumpSwap** | Solana AMM graduated tokens | RPC + Ed25519 |

### MetaTrader 5 (via Python bridge)

| Markets | Instruments |
|---------|-------------|
| **Forex** | EURUSD, GBPUSD, USDJPY, 50+ pairs |
| **Gold & Metals** | XAUUSD, XAGUSD |
| **Indices** | US500, US30, GER40 |
| **Stocks** | AAPL, TSLA, AMZN (via MT5 broker) |
| **Crypto CFD** | BTCUSD, ETHUSD |

### CCXT (via Python bridge — 100+ exchanges)

| Exchange | Type | | Exchange | Type |
|----------|------|-|----------|------|
| **Bybit** | Spot + Futures | | **Gate.io** | Spot + Futures |
| **OKX** | Spot + Futures | | **KuCoin** | Spot + Futures |
| **Kraken** | Spot + Margin | | **Bitget** | Spot + Futures |
| **Coinbase** | Spot | | **MEXC** | Spot + Futures |
| **HTX** | Spot + Futures | | **[90+ more](https://github.com/ccxt/ccxt)** | All types |

> **One API, any market.** Your agent calls `POST /trade` — GreedyClaw routes to the right exchange.

---

## Security

GreedyClaw implements **6 defense layers**. Every layer is active by default. There is no "disable security" flag.

### Layer 1: Network isolation

- Binds to `127.0.0.1` only — invisible on the network
- No mDNS broadcast, no admin panel, no discovery
- Zero attack surface from outside the machine

### Layer 2: Authentication

- `Authorization: Bearer <token>` on **every** request (no exceptions)
- **Constant-time comparison** (`subtle` crate) — prevents timing side-channel attacks
- **Crypto-random 64-char hex token** generated on `greedyclaw init` (`rand` crate)
- Token stored in `~/.greedyclaw/.env` — never in URLs, never in browser storage
- 1MB request body limit — prevents OOM from oversized payloads

### Layer 3: Input validation

- NaN, Infinity, negative values **rejected** before reaching exchange
- $1B sanity cap on quantity and price — prevents f64 overflow exploits
- Symbol length capped at 20 chars — no oversized string attacks
- Order type, side, amount validated with LLM-friendly error messages

### Layer 4: Financial safety (Risk Engine)

- **Mandatory and cannot be disabled** — this is architectural, not configurable
- Symbol whitelist, position limits, daily loss kill switch
- Hallucination loop detector — rate limiter returns `429` with explanation
- Mark-to-market floating PnL tracked in real-time
- See [Risk Engine](#risk-engine) for details

### Layer 5: Audit integrity

- **HMAC-SHA256 signature** on every audit entry (key derived from auth token)
- SQLite WAL + JSONL dual-write with `fsync` — crash-safe
- Agent cannot modify its own audit trail
- Full risk snapshot captured with each trade

### Layer 6: Error sanitization

- Internal error details **never leaked** to clients
- Client receives safe message + error code + actionable suggestion
- Full error logged server-side only
- Exchange API keys never leave Gateway process; LLM API keys never sent to Gateway

### Brain-level security

- **SSRF protection** — `fetch_url` blocks private/internal IPs (127.x, 10.x, 192.168.x, 169.254.x, metadata endpoints)
- **Confidence gate** — trades blocked in code if `log_decision` confidence < 70%
- **API key hygiene** — Google API key in header, not URL

### Comparison with typical AI frameworks

| Threat | Typical AI Frameworks | GreedyClaw |
|--------|----------------------|------------|
| **Remote Code Execution** | Admin UI exposed to network. 15K+ vulnerable instances found | No admin UI. REST API only, `127.0.0.1`. Zero network attack surface |
| **Supply Chain Attack** | Plugin marketplaces with unverified extensions | No marketplace. Skills are local `.md` files you control |
| **Prompt Injection → Account Drain** | LLM has shell access + exchange keys | Brain has no shell, no keys, no direct exchange access. Trades pass through risk engine |
| **Hallucination Loop** | No rate limits. AI retries failed trades infinitely | Rate limiter detects loops → `429 RATE_LIMIT` → circuit breaker |
| **Credential Leak** | Cloud DBs, localStorage, URL query strings | Local `.env` only. Constant-time auth. Token never in URLs |
| **Audit Tampering** | No audit trail, or mutable logs | HMAC-SHA256 signed entries. SQLite + JSONL dual-write with fsync |

---

## Risk Engine

The risk engine runs inside the Rust Gateway. It is **mandatory and cannot be disabled**. Every trade passes through it — no exceptions, no bypass.

| Protection | Default | Description |
|------------|---------|-------------|
| **Symbol whitelist** | Configurable | Only trade explicitly allowed pairs |
| **Max position size** | $500 | Cap single-trade USD exposure |
| **Max daily loss** | $100 | Kill switch — stops ALL trading when hit |
| **Max open positions** | 3 | Prevent over-diversification |
| **Rate limiter** | 10/min | Circuit breaker for hallucination loops |
| **Input validation** | Always on | Reject NaN, Infinity, negatives, $1B+ amounts |
| **Floating PnL** | Real-time | Mark-to-market tracking for accurate daily P&L |

### How the kill switch works

```
Pre-trade check:
  1. Rate limit     → 429 "Possible hallucination loop"
  2. Symbol check   → 403 "Not in allowed list"
  3. Input sanity   → 400 "Must be finite positive number"
  4. Position size   → 403 "Exceeds max $500"
  5. Open positions → 403 "3 open, max is 3"
  6. Daily loss     → 403 "Daily PnL -$102 exceeds -$100"

ALL checks pass → execute trade → record fill → update PnL
```

---

## MCP Server

Connect Claude Desktop, Cursor, or VS Code directly to GreedyClaw via [Model Context Protocol](https://modelcontextprotocol.io/).

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

### 12 MCP tools

| Tool | | Tool | |
|------|-|------|-|
| `trade` | Execute buy/sell | `status` | Gateway health + risk |
| `balance` | Account balances | `positions` | Open positions + PnL |
| `price` | Current price | `trades` | Recent history |
| `stats` | Trade statistics | `pnl` | PnL time series |
| `cancel` | Cancel order | `scanner_start` | Start token scanner |
| `scanner_stop` | Stop scanner | `scanner_status` | Scanner metrics |

---

## Scanner

Built-in **PumpFun token scanner** — streams Solana transactions via Yellowstone gRPC, scores tokens in real-time using the LAZARUS strategy (Optuna-optimized parameters).

- Real-time bonding curve tracking
- Anti-rug filters: whale detection, sell ratio analysis, zombie token filtering
- Configurable trigger parameters via REST API
- Visual dashboard with live token metrics

```bash
# Start via API
curl -X POST http://127.0.0.1:7878/scanner/start \
  -H "Authorization: Bearer your_token"

# Check discovered tokens
curl http://127.0.0.1:7878/scanner/status \
  -H "Authorization: Bearer your_token"
```

---

## Docker

```bash
cp .env.example .env && nano .env

docker-compose up                                     # Gateway + Brain
docker-compose --profile mt5 up                       # + MT5 bridge
CCXT_EXCHANGE=bybit docker-compose --profile ccxt up  # + CCXT bridge
```

| Service | Port | Description |
|---------|------|-------------|
| `gateway` | 7878 | Rust execution gateway |
| `brain` | — | Autonomous AI agent |
| `mt5-bridge` | 7879 | MT5 connector (optional) |
| `ccxt-bridge` | 7880 | CCXT connector (optional) |

---

## API

All endpoints require `Authorization: Bearer <token>` (except `/dashboard`).

### Trading

```
POST   /trade         Execute buy/sell (market or limit)
DELETE /order/{id}     Cancel open order
```

### Account & monitoring

```
GET /status            Health + risk engine snapshot
GET /balance           Account balances
GET /positions         Open positions + unrealized PnL
GET /price/{symbol}    Current price
GET /trades            Recent trades from audit log
GET /trades/stats      Aggregated trade statistics
GET /trades/pnl        PnL time series (equity curve)
GET /dashboard         Visual trading dashboard (no auth)
```

### Scanner

```
POST /scanner/start       Start gRPC token scanner
POST /scanner/stop        Stop scanner
GET  /scanner/status      Scanner metrics + top tokens
GET  /scanner/tokens      All tracked tokens
GET  /scanner/config      Get scanner config
PUT  /scanner/config      Update scanner config
GET  /scanner/positions   Scanner-managed positions
```

### Example: Python agent

```python
import requests

GW = "http://127.0.0.1:7878"
H = {"Authorization": "Bearer your_token"}

# Buy gold on MT5
requests.post(f"{GW}/trade", headers=H, json={"action": "buy", "symbol": "XAUUSD", "amount": 0.01})

# Check risk status
requests.get(f"{GW}/status", headers=H).json()
# → {"risk": {"daily_pnl": -12.50, "remaining_daily_limit": 87.50, "open_positions": 1, ...}}
```

### Example: Claude / GPT function calling

```json
{
  "name": "execute_trade",
  "description": "Execute a trade via GreedyClaw gateway with mandatory risk checks.",
  "parameters": {
    "type": "object",
    "properties": {
      "action": {"type": "string", "enum": ["buy", "sell"]},
      "symbol": {"type": "string"},
      "amount": {"type": "number"}
    },
    "required": ["action", "symbol", "amount"]
  }
}
```

---

## Configuration

### `~/.greedyclaw/config.toml`

```toml
[server]
host = "127.0.0.1"
port = 7878

[exchange]
name = "binance"       # or "mt5", "bybit", "okx", "pumpfun", etc.
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
symbols: [XAUUSD]
sources: [forex_factory, investing_com]
```

### `~/.greedyclaw/.env`

```env
GREEDYCLAW_AUTH_TOKEN=<auto-generated 64-char hex>

# Exchange (set one)
BINANCE_API_KEY=...
BINANCE_SECRET_KEY=...

# LLM (set at least one for Brain)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
GOOGLE_API_KEY=AI...
```

---

## Using MT5 (Forex, Gold, Indices)

```bash
cd mt5-bridge && pip install -r requirements.txt
python mt5_bridge.py                # Starts on :7879
# Set exchange = "mt5" in config.toml
greedyclaw serve
greedyclaw trade buy XAUUSD 0.01    # Buy 0.01 lot gold
```

## Using CCXT (Bybit, OKX, Kraken, 100+ more)

```bash
cd mt5-bridge && pip install ccxt fastapi uvicorn
CCXT_API_KEY=... CCXT_SECRET=... python ccxt_bridge.py --exchange bybit
# Set exchange = "bybit" in config.toml
greedyclaw serve
```

---

## Roadmap

- [x] **Phase 1** — Binance Testnet, REST API, risk engine, audit log
- [x] **Phase 2** — PumpFun + PumpSwap (Solana, Ed25519 signing, Jupiter)
- [x] **Phase 3** — Visual trading dashboard, PnL charts
- [x] **Phase 4** — PumpFun token scanner (LAZARUS strategy, Yellowstone gRPC)
- [x] **Phase 5** — MT5 + CCXT (100+ exchanges, forex, gold, stocks)
- [x] **Phase 8** — MCP Server (12 tools for Claude/Cursor/VS Code)
- [x] **Phase 9** — AI Brain (autonomous agent, 6 LLM providers, skills)
- [x] **Phase 10** — Security hardening (constant-time auth, HMAC audit, SSRF protection)
- [x] **Docker** — One-command deployment
- [ ] **Phase 6** — Auto-trade (scanner triggers wired to Brain)
- [ ] **Phase 7** — WebSocket (real-time feeds + fill notifications)
- [ ] **Phase 11** — Strategy SDK (pluggable modules with backtesting)
- [ ] **Phase 12** — Telegram Bot (mobile notifications + control)

## Contributing

```bash
git clone https://github.com/GreedyClaw/GreedyClaw.git
cd GreedyClaw && cargo build && cargo test
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).

---

<p align="center">
  <img src="src/clawicon.png" width="60"/>
  <br/>
  <strong>Built with Rust. Powered by AI. Secured by design.</strong>
</p>
