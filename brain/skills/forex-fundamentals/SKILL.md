---
name: forex-fundamentals
description: "Trade major forex pairs based on fundamental analysis, rate differentials, and economic data. Use when: market is forex, symbols include EURUSD/GBPUSD/USDJPY/AUDUSD, or high-impact economic events are driving currency moves."
symbols: [EURUSD, GBPUSD, USDJPY, AUDUSD, XAUUSD]
timeframes: [H1, H4, D1]
---

# Forex Fundamental Analysis Trading

## Overview
Forex prices are driven by interest rate differentials, economic growth divergence,
and capital flows. This skill provides rules for trading major pairs based on
fundamental factors — not technical indicators. The edge comes from correctly reading
central bank policy, economic data surprises, and carry dynamics.

---

## STEP 1: Gather Data (use tools)

Run these searches at the start of every cycle.

### A. Economic Calendar (CRITICAL)
- `web_search("economic calendar today high impact forex factory")` — today's events
- `web_search("economic calendar this week high impact")` — upcoming events

Classify each event by impact:
| Event Type  | Impact | Pairs Affected |
|-------------|--------|----------------|
| NFP         | EXTREME| All USD pairs  |
| CPI (US)    | HIGH   | All USD pairs  |
| FOMC/Fed    | EXTREME| All USD pairs  |
| ECB decision| HIGH   | EURUSD, EURGBP |
| BOE decision| HIGH   | GBPUSD, EURGBP |
| BOJ decision| HIGH   | USDJPY         |
| RBA decision| HIGH   | AUDUSD         |
| GDP (any G7)| MEDIUM | Respective pair|
| PMI (any G7)| MEDIUM | Respective pair|
| Retail Sales| MEDIUM | Respective pair|
| Trade Balance| LOW   | Respective pair|

### B. Rate Differentials
- `web_search("central bank interest rates 2026 comparison")` — current rates
- `web_search("Fed funds rate expectations CME FedWatch")` — forward expectations
- `web_search("ECB deposit rate expectations")` — EUR rate outlook
- `web_search("BOJ interest rate policy")` — JPY rate outlook

### C. Growth Divergence
- `web_search("US GDP growth forecast 2026")` — US trajectory
- `web_search("eurozone GDP growth forecast 2026")` — EUR trajectory
- `web_search("UK GDP growth forecast 2026")` — GBP trajectory

### D. Current Account State
- `get_positions()` — existing positions
- `get_balance()` — margin available
- `get_risk_status()` — daily PnL and limits

---

## STEP 2: Pair-Specific Fundamental Analysis

### EUR/USD
**Primary driver**: Fed vs ECB rate differential
- Fed hawkish + ECB dovish → SELL EURUSD (USD strength)
- Fed dovish + ECB hawkish → BUY EURUSD (EUR strength)
- Both hawkish or both dovish → look at DEGREE of hawkishness

**Secondary drivers**:
- Eurozone current account surplus → structural EUR support
- US fiscal deficit widening → medium-term USD negative
- Risk-on → EUR positive (EUR is a growth currency)
- Risk-off → EUR negative vs USD

**Key data to watch**: ECB speeches, German PMI, US CPI, NFP

### GBP/USD
**Primary driver**: BOE rate expectations vs Fed
- BOE hiking/holding while Fed cutting → BUY GBPUSD
- BOE cutting while Fed holding → SELL GBPUSD

**Secondary drivers**:
- UK inflation (persistently high → BOE forced to stay tight → GBP bid)
- UK political risk → GBP negative
- UK trade balance (chronic deficit → structural GBP headwind)

**Key data**: UK CPI, BOE minutes, UK employment, UK retail sales

### USD/JPY
**Primary driver**: US-Japan rate differential (the carry trade)
- Wide US-Japan spread (US rates >> Japan rates) → BUY USDJPY (carry)
- BOJ tightening signals → SELL USDJPY (carry unwind)
- Risk-off → SELL USDJPY (JPY safe haven, carry unwind)

**WARNING**: USDJPY carry unwinds are VIOLENT. When they start, they move 500-1000 pips.
- If BOJ announces policy shift: SELL USDJPY immediately, do not wait for confirmation.
- If VIX spikes > 30 and USDJPY is falling: SELL or close longs immediately.

**Secondary drivers**:
- Japan intervention threats (verbal at 150+, actual at 155+) → reversal risk
- US Treasury yields (direct correlation with USDJPY)

**Key data**: BOJ meetings, Japan CPI, US NFP, US CPI, risk events

### AUD/USD
**Primary driver**: China growth + commodity demand
- China PMI expanding + iron ore prices rising → BUY AUDUSD
- China slowdown + commodity prices falling → SELL AUDUSD

**Secondary drivers**:
- RBA vs Fed rate differential
- Risk appetite (AUD is a high-beta risk currency)
- Australia employment data

**Key data**: China PMI, RBA meetings, AU employment, iron ore prices

---

## STEP 3: Score the Fundamental Picture

For each pair you are considering trading, score these 4 dimensions:

**A. Rate Differential Direction (weight: 2.0x)**
The single most important driver of FX.
- Differential widening strongly in favor of base currency → +2
- Differential widening slightly → +1
- Stable → 0
- Differential narrowing slightly → -1
- Differential narrowing strongly / pivot → -2

**B. Economic Growth Divergence (weight: 1.5x)**
Relative growth matters more than absolute growth.
- Base currency economy clearly outperforming → +2
- Slightly outperforming → +1
- Similar growth → 0
- Base currency economy underperforming → -1
- Recession risk in base currency economy → -2

**C. Data Surprise Direction (weight: 1.5x)**
Recent economic data relative to consensus forecasts.
- Multiple strong positive surprises for base currency → +2
- One positive surprise → +1
- Inline with forecasts → 0
- One negative surprise → -1
- Multiple negative surprises → -2

**D. Risk Sentiment Alignment (weight: 1.0x)**
Does the current risk environment favor this pair?
- Risk environment strongly favors base currency → +2
- Slightly favors → +1
- Neutral → 0
- Slightly negative → -1
- Strongly negative → -2

Note: "base currency" here means the first currency in the pair (EUR in EURUSD).
A positive score means BUY the pair. A negative score means SELL the pair.

### Compute Weighted Score

```
weighted_score = (A * 2.0 + B * 1.5 + C * 1.5 + D * 1.0) / 6.0
```

---

## STEP 4: Event Trading Rules

### Pre-Event (event in next 0-2 hours)
- DO NOT open new positions in affected pairs
- Existing positions: tighten stops or close if near breakeven
- Log: "Event filter: [EVENT] in [X] hours for [PAIR], holding"

### Post-Event (data just released, within 30 minutes)

#### Reading the Data Release
The DEVIATION (actual - forecast) drives the move, not the absolute number.

| Data Point     | Positive Surprise for USD | Negative Surprise for USD |
|----------------|---------------------------|---------------------------|
| NFP            | actual > forecast + 50K   | actual < forecast - 50K   |
| CPI (MoM)     | actual > forecast + 0.1%  | actual < forecast - 0.1%  |
| GDP (QoQ)      | actual > forecast + 0.3%  | actual < forecast - 0.3%  |
| PMI            | actual > forecast + 1.0   | actual < forecast - 1.0   |
| Retail Sales   | actual > forecast + 0.3%  | actual < forecast - 0.3%  |

These thresholds define "meaningful surprise." Deviations within threshold = noise.

#### Post-Release Trading Protocol
1. Wait 15-30 minutes after release (initial spike is noise + algo wars)
2. Check if price has SUSTAINED the move or reversed
3. If sustained: trade in direction of the sustained move
4. If reversed: the surprise was already priced in — fade the reversal
5. Only trade if the deviation exceeds the threshold above

#### FOMC / Central Bank Meetings
These are different — they set the MEDIUM TERM direction, not just a spike.
1. Read the statement carefully (search for "FOMC statement analysis")
2. Hawkish shift = buy the currency. Dovish shift = sell.
3. The press conference matters as much as the statement
4. Wait for the press conference to finish before trading
5. The move after FOMC tends to extend for 2-3 days — hold positions longer

---

## STEP 5: Carry Trade Rules

Carry trades (buy high-yield currency, sell low-yield currency) work in LOW volatility.

### When to Enter Carry Trades
- VIX < 18 AND trending lower
- Rate differential > 200bps
- No major central bank meetings in next 2 weeks
- No geopolitical escalation

### Carry Trade Pairs (typical, verify current rates)
- Long USDJPY (if US rates >> Japan rates)
- Long AUDUSD (if RBA rates >> Fed rates) — rare currently
- Long GBPUSD (if BOE rates >> Fed rates)

### When to EXIT Carry Trades Immediately
- VIX spikes above 25
- Central bank surprise (especially BOJ)
- Geopolitical escalation
- Rate differential narrows suddenly (surprise cut)

**Carry unwinds are the most violent moves in FX. Exit fast, do not hold and hope.**

---

## STEP 6: Confidence Scoring

| |Weighted Score| | Base Confidence |
|------------------|-----------------|
| >= 1.5           | 90              |
| 1.0 - 1.49       | 85              |
| 0.75 - 0.99      | 80              |
| 0.5 - 0.74       | 75              |
| < 0.5            | < 70 = HOLD     |

### Adjustments
- Rate differential AND data surprise aligned: +5
- Post-event trade with clear deviation: +5
- Central bank pivot (major directional shift): +5
- Conflicting drivers (rate vs growth divergence): -10
- Event in next 2 hours: -15
- Weekend approaching (Friday after 18:00 UTC): -10
- Spread wider than normal (check get_price): -5

---

## STEP 7: Position Sizing

| Confidence | Lot Size (EURUSD/GBPUSD) | Lot Size (USDJPY) | Lot Size (AUDUSD) |
|------------|--------------------------|--------------------|--------------------|
| 90-95      | 0.03                     | 0.03               | 0.02               |
| 85-89      | 0.02                     | 0.02               | 0.015              |
| 80-84      | 0.01                     | 0.01               | 0.01               |
| 75-79      | 0.01                     | 0.01               | 0.01               |
| < 75       | 0 (HOLD)                 | 0 (HOLD)           | 0 (HOLD)           |

### Scaling In
For high-conviction fundamental themes (confidence >= 85):
- Enter 50% of position size on first signal
- Add remaining 50% if price moves in your direction after 4-8 hours
- Never add to a losing position

### Hard Limits
- Maximum 2 forex positions open simultaneously
- Maximum 1 position per pair
- Maximum total forex exposure: 0.05 lots across all pairs
- If daily PnL loss > 1.5% of equity: STOP forex trading for the day

---

## STEP 8: Stop Loss and Take Profit

| Pair     | Typical SL (pips) | TP Target       | Notes                          |
|----------|--------------------|-----------------|--------------------------------|
| EURUSD   | 40-60              | 80-150 pips     | Tight range? Use 40 SL.       |
| GBPUSD   | 50-70              | 100-175 pips    | GBP is more volatile.         |
| USDJPY   | 50-80              | 100-200 pips    | Wider in carry unwind.         |
| AUDUSD   | 40-60              | 80-150 pips     | Correlates with risk appetite. |
| XAUUSD   | $8-$15             | $16-$40         | SL >= 1.0 ATR MINIMUM.        |

### Risk-Reward Rules
- Minimum R:R ratio: 1.5:1
- Preferred R:R ratio: 2.0:1 to 3.0:1
- If you cannot find an entry where R:R >= 1.5:1, do not trade

### Fundamental Exits (override technical stops)
- Central bank pivot against your position → close immediately
- Major data surprise against your position → close or tighten stop to breakeven
- The fundamental thesis that justified entry is no longer valid → close

---

## STEP 9: Execute

1. Call `log_decision` with:
   - symbol: the pair
   - action: "buy" / "sell" / "hold"
   - confidence: computed value
   - reasoning: Include all 4 driver scores, weighted score, event check, pair-specific analysis
   - sources: search queries and key data points

2. If confidence >= 70 AND no event filter AND session is valid (not weekend):
   - Call `trade(action, symbol, lot_size)`

3. Otherwise:
   - Log as HOLD with full reasoning
   - Do NOT call trade

---

## STEP 10: Multi-Pair Correlation Check

Before opening a new position, check for correlation conflicts:

| If you hold...  | Do NOT also hold... | Reason                    |
|------------------|---------------------|---------------------------|
| Long EURUSD      | Long GBPUSD         | Both are short USD bets   |
| Short EURUSD     | Short GBPUSD        | Both are long USD bets    |
| Long USDJPY      | Short EURUSD        | Both are long USD bets    |
| Long AUDUSD      | Long EURUSD         | Both are risk-on + short USD |
| Long XAUUSD      | Long EURUSD         | Both are anti-USD trades  |

**Rule**: Maximum 2 correlated positions. If you already have 2 USD-negative trades,
do not add a third. This prevents blow-up on a single USD move.

---

## Session Timing

| Session         | UTC         | Best For                                      |
|-----------------|-------------|-----------------------------------------------|
| Tokyo           | 00:00-08:00 | USDJPY only. Avoid EURUSD/GBPUSD (illiquid).  |
| London          | 07:00-16:00 | ALL pairs. Best liquidity.                     |
| NY overlap      | 12:00-16:00 | ALL pairs. Highest volume.                     |
| NY afternoon    | 16:00-21:00 | USD pairs acceptable. EUR/GBP widening.        |
| Weekend gap     | Fri 21:00+  | CLOSE or reduce all positions before weekend.  |

---

## Anti-Patterns (NEVER DO THESE)

1. **Do NOT trade data releases before they print**. You are not a prophet.
2. **Do NOT hold carry trades through a VIX spike**. Carry unwinds are brutal.
3. **Do NOT ignore the rate differential**. It is the gravity of FX.
4. **Do NOT fight a central bank pivot**. If the Fed pivots dovish, sell USD.
5. **Do NOT scale into losing positions**. If the thesis is wrong, cut.
6. **Do NOT hold GBPUSD through UK political chaos without a wide stop**.
7. **Do NOT trade USDJPY on the assumption BOJ will never tighten**. They will, and it will move 1000 pips.
8. **Do NOT trade based on a single data point**. Require theme confirmation.
9. **Do NOT hold positions over the weekend without a strong reason**. Gap risk.
10. **Do NOT trade when you cannot clearly state the fundamental thesis in one sentence**.

---

## Example Reasoning (for log_decision)

```
"EURUSD fundamental analysis:
Rate Differential: +1 (ECB holding 3.75% vs Fed at 4.50%, but market pricing
2 Fed cuts in next 6 months vs 0 ECB cuts — differential narrowing toward EUR).
Growth Divergence: -1 (US GDP 2.1% vs Eurozone 0.8%, US clearly outperforming).
Data Surprise: +1 (Eurozone PMI surprised at 52.3 vs 50.8 forecast yesterday;
US ISM missed at 48.2 vs 49.5 forecast).
Risk Sentiment: 0 (VIX 17, neutral, no clear risk-on/off).
Weighted: (1*2.0 + (-1)*1.5 + 1*1.5 + 0*1.0) / 6.0 = +0.33.
Score < 0.5 threshold. HOLD.
Growth divergence offsets rate expectations. Need more data alignment.
Session: London (10:15 UTC). No events next 2 hours. Would be valid if score was higher.
Decision: HOLD EURUSD — insufficient fundamental alignment."
```
