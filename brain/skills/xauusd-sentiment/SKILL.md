---
name: xauusd-sentiment
description: "Trade XAUUSD based on macro sentiment, fundamentals, and session timing. Use when: gold trading, sentiment-driven approach, macro analysis, or XAUUSD is in the symbol list."
symbols: [XAUUSD]
timeframes: [H1, H4, D1]
---

# XAUUSD Sentiment & Macro Trading

## Overview
Gold is a macro asset. It moves on real yields, USD strength, central bank flows, and
geopolitical risk. This skill provides concrete rules for trading XAUUSD based on
synthesizing these drivers into an actionable signal.

---

## STEP 1: Gather Data (use tools)

Run these searches EVERY cycle. Do not skip any.

1. **Gold spot + macro snapshot**
   - `web_search("XAUUSD gold price today Reuters")` — current price action and context
   - `web_search("DXY dollar index today")` — USD strength (inverse correlation with gold)
   - `web_search("US 10 year real yield TIPS today")` — the single best predictor of gold

2. **Central bank and policy**
   - `web_search("Federal Reserve statement latest")` — hawkish/dovish lean
   - `web_search("ECB rate decision latest")` — EUR/USD impacts gold via DXY
   - `web_search("central bank gold purchases 2026")` — structural demand floor

3. **Geopolitics and risk**
   - `web_search("geopolitical risk gold safe haven")` — wars, sanctions, elections
   - `web_search("VIX today stock market risk")` — risk-off = gold bid

4. **Economic calendar**
   - `web_search("economic calendar today high impact")` — NFP, CPI, FOMC, GDP
   - Check if data has been RELEASED (actual vs forecast) or is UPCOMING

5. **Current account state**
   - `get_price("XAUUSD")` — current bid/ask and spread
   - `get_positions()` — check if already positioned
   - `get_balance()` — available margin
   - `get_risk_status()` — daily PnL, exposure limits

---

## STEP 2: Score Each Driver (-2 to +2)

After gathering data, score each factor on this scale:

| Score | Meaning |
|-------|---------|
| +2    | Strongly bullish for gold (e.g., Fed cutting rates, war escalation) |
| +1    | Mildly bullish (e.g., DXY weakening, yields dipping) |
|  0    | Neutral or no clear signal |
| -1    | Mildly bearish (e.g., strong USD, rising yields) |
| -2    | Strongly bearish (e.g., hawkish Fed surprise, risk-on euphoria) |

### The 5 Drivers

**A. Real Yields (weight: 1.5x)**
Gold's primary driver. When real yields rise, gold falls (opportunity cost of holding
non-yielding asset increases). When real yields fall, gold rises.
- US 10Y TIPS yield rising > 20bps in a week → score -2
- US 10Y TIPS yield rising 5-20bps → score -1
- Stable → score 0
- Falling 5-20bps → score +1
- Falling > 20bps or going negative → score +2

**B. USD Strength / DXY (weight: 1.25x)**
Gold is priced in USD. Strong dollar = headwind. Weak dollar = tailwind.
- DXY above 105 and rising → score -2
- DXY 100-105 and rising → score -1
- DXY stable or range-bound → score 0
- DXY falling from 100-105 → score +1
- DXY below 100 and falling → score +2

**C. Central Bank Policy (weight: 1.25x)**
Fed policy sets the tone. Other central banks matter less but ECB and BOJ affect DXY.
- Fed hawkish surprise (higher-for-longer, rate hike signal) → score -2
- Fed modestly hawkish (dot plot up, taper talk) → score -1
- Fed on hold, no change → score 0
- Fed dovish tilt (acknowledging slowing, pause signal) → score +1
- Fed cutting or signaling imminent cuts → score +2

**D. Geopolitical Risk (weight: 1.0x)**
Gold is the ultimate safe haven. Geopolitical escalation drives flight to gold.
- Peace deal / de-escalation → score -1
- No significant geopolitical events → score 0
- Moderate tension (sanctions, elections, trade disputes) → score +1
- Active military conflict or major escalation → score +2

**E. Market Sentiment / Risk Appetite (weight: 1.0x)**
VIX, equity markets, and general risk appetite.
- VIX < 15, equities at highs, euphoria → score -1
- Normal conditions → score 0
- VIX 20-30, equities falling → score +1
- VIX > 30, panic selling, risk-off → score +2

### Compute Weighted Score

```
weighted_score = (A * 1.5 + B * 1.25 + C * 1.25 + D * 1.0 + E * 1.0) / 6.0
```

The denominator is 6.0 (sum of weights), normalizing the score to the -2 to +2 range.

---

## STEP 3: Map Score to Signal

| Weighted Score       | Signal  | Action                |
|----------------------|---------|-----------------------|
| >= +1.0              | BUY     | Enter long            |
| +0.5 to +0.99       | BUY     | Enter long (smaller)  |
| -0.49 to +0.49      | HOLD    | No trade              |
| -0.99 to -0.5       | SELL    | Enter short           |
| <= -1.0             | SELL    | Enter short (larger)  |

---

## STEP 4: Session Filter (MANDATORY)

**CRITICAL: Do NOT trade outside these windows.**

| Session            | UTC Hours  | Rule                                        |
|--------------------|------------|---------------------------------------------|
| London             | 07:00-16:00| BEST. Highest liquidity, tightest spreads.  |
| New York overlap   | 12:00-16:00| Also good. Most volume in gold.             |
| Early NY           | 16:00-20:00| Acceptable but spreads widening.            |
| Asian              | 22:00-07:00| DO NOT TRADE. Spreads $0.50-$1.00+.         |

If the current UTC time is outside 07:00-20:00, the signal is forced to HOLD regardless
of the score. Log: "Session filter: Asian session, holding."

---

## STEP 5: Event Proximity Filter

If a HIGH IMPACT event (NFP, CPI, FOMC) is within the NEXT 2 HOURS:
- DO NOT open new positions. The move will be random until the data prints.
- If already positioned, tighten mental stop or close.
- Log: "Event filter: [EVENT] in [X] hours, holding."

If a HIGH IMPACT event was released in the LAST 30 MINUTES:
- The initial spike is noise. Wait 15-30 minutes for the market to digest.
- Then trade the direction of the sustained move, not the spike.

---

## STEP 6: Confidence Scoring

Map the absolute value of the weighted score to confidence:

| |Weighted Score| | Base Confidence |
|------------------|-----------------|
| >= 1.5           | 90              |
| 1.0 - 1.49       | 85              |
| 0.75 - 0.99      | 80              |
| 0.5 - 0.74       | 75              |
| < 0.5            | < 70 = HOLD     |

### Confidence Adjustments (add/subtract from base)
- Multiple drivers aligned in same direction: +5
- Strong volume / price momentum confirming: +5
- Conflicting signals between drivers: -10
- Low liquidity session (early Asian overlap): -10
- Major event in next 2 hours: -15 (forces HOLD in most cases)
- Spread > $0.50 from get_price: -5

Final confidence is capped at 95 and floored at 0.

---

## STEP 7: Position Sizing

| Confidence | Lot Size |
|------------|----------|
| 90-95      | 0.02     |
| 85-89      | 0.015    |
| 80-84      | 0.01     |
| 75-79      | 0.01     |
| 70-74      | 0.01     |
| < 70       | 0 (HOLD) |

### Hard Limits
- Maximum position: 0.03 lots total (including existing positions)
- Maximum 2 open XAUUSD positions at any time
- If `get_risk_status()` shows daily loss > 2% of equity, STOP trading for the day

---

## STEP 8: Stop Loss and Take Profit

### CRITICAL: SL < 0.45 ATR IS DEAD ON REAL MARKET

Gold has $0.50-$0.80 typical slippage. Tight stops get stopped out on noise.

| Parameter       | Rule                         |
|-----------------|------------------------------|
| Stop Loss       | >= 1.0 ATR (H1), minimum $8  |
| Take Profit     | 2.0-3.0x the SL distance     |
| Trailing Stop   | After 1.5x SL in profit, trail at 1.0 ATR |

ATR reference for XAUUSD H1: typically $6-$15 depending on volatility regime.

If you cannot determine ATR from available data, use these defaults:
- SL: $12 from entry (approximately 1.0 ATR in normal conditions)
- TP: $30 from entry (approximately 2.5x SL, risk-reward 2.5:1)

**NEVER set SL tighter than $5 from entry. This is a hard floor.**

---

## STEP 9: Execute (use tools)

1. Call `log_decision` with:
   - symbol: "XAUUSD"
   - action: "buy" / "sell" / "hold"
   - confidence: (computed value)
   - reasoning: Include ALL 5 driver scores, weighted score, session check, event check
   - sources: List of search queries and key findings

2. If confidence >= 70 AND session is valid AND no event filter:
   - Call `trade(action, "XAUUSD", lot_size)`

3. If confidence < 70 OR session invalid OR event filter active:
   - Log as HOLD with full reasoning
   - Do NOT call trade

---

## STEP 10: Position Management (if already positioned)

Check `get_positions()` each cycle. For existing XAUUSD positions:

- **In profit > 2x SL distance**: Consider partial close (close half) or tighten trail
- **At breakeven after 4+ hours**: The trade is stale. Close at breakeven.
- **Hitting SL**: Let it trigger. Do not widen SL after entry.
- **Fundamental shift**: If the macro picture reverses (e.g., surprise hawkish Fed when
  you are long), close immediately regardless of PnL.

---

## Anti-Patterns (NEVER DO THESE)

1. **Do NOT trade the Asian session**. Spreads eat your edge.
2. **Do NOT set SL < 0.45 ATR**. Slippage will stop you out 100% of the time.
3. **Do NOT trade NFP/CPI spike**. Wait 15-30 minutes for digestion.
4. **Do NOT trade on a single driver**. Require 2+ drivers aligned.
5. **Do NOT average down** on a losing gold position. Gold trends hard.
6. **Do NOT trade if you cannot clearly state your thesis** in one sentence.
7. **Do NOT trade gold when DXY and yields are sending conflicting signals** — wait.
8. **Do NOT override the confidence threshold**. If it says HOLD, hold.

---

## Example Reasoning (for log_decision)

```
"Driver scores: Real Yields +1 (10Y TIPS falling 8bps this week),
DXY -1 (dollar at 104.2, rising), Central Bank +1 (Fed dovish pause),
Geopolitics +1 (Middle East tension elevated), Sentiment 0 (VIX 18, neutral).
Weighted: (1*1.5 + (-1)*1.25 + 1*1.25 + 1*1.0 + 0*1.0) / 6.0 = +0.42.
Score < 0.5 threshold. Signal: HOLD. DXY headwind offsets dovish Fed + geopolitics.
Session: London (14:22 UTC) — valid. No high-impact events next 2 hours.
Decision: HOLD — insufficient alignment across drivers."
```
