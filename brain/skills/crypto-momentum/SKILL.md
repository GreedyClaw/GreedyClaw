---
name: crypto-momentum
description: "Trade BTC and ETH based on momentum, on-chain data, funding rates, and social sentiment. Use when: market is crypto, symbols include BTC/ETH/BTCUSDT/ETHUSDT, or crypto-specific signals detected."
symbols: [BTCUSDT, ETHUSDT]
timeframes: [H1, H4, D1]
---

# Crypto Momentum Trading

## Overview
Crypto markets are momentum-driven with higher volatility (3-5x forex), 24/7 trading,
and unique data sources (on-chain, funding rates, ETF flows). This skill provides rules
for trading BTC and ETH based on momentum signals, sentiment extremes, and on-chain data.
The edge comes from correctly reading momentum exhaustion and contrarian extremes.

---

## STEP 1: Gather Data (use tools)

Run ALL of these searches every cycle. Crypto moves fast — stale data kills.

### A. Price Action & Momentum
- `get_price("BTCUSDT")` — current bid/ask
- `get_price("ETHUSDT")` — current bid/ask
- `web_search("bitcoin price today analysis")` — trend context
- `web_search("ethereum price today analysis")` — ETH-specific moves

### B. Sentiment & Fear/Greed
- `web_search("crypto fear greed index today")` — 0-100 scale
  - 0-24: Extreme Fear (contrarian BUY zone)
  - 25-44: Fear (lean BUY)
  - 45-55: Neutral
  - 56-74: Greed (lean SELL / take profits)
  - 75-100: Extreme Greed (contrarian SELL zone)
- `web_search("bitcoin social sentiment twitter")` — retail euphoria/panic gauge

### C. On-Chain & Flow Data
- `web_search("bitcoin exchange inflows outflows today")` — exchange flows
  - Large inflows to exchanges = selling pressure incoming
  - Large outflows from exchanges = accumulation (bullish)
- `web_search("bitcoin whale transactions today")` — whale movements
- `web_search("bitcoin ETF flows today")` — institutional flows (BTC only)
  - Positive ETF flows = institutional demand = bullish
  - Negative ETF flows = institutional selling = bearish

### D. Derivatives Data
- `web_search("bitcoin funding rate today")` — perpetual futures funding
  - Positive > 0.05%: Longs paying shorts. Market overheated long. Contrarian bearish.
  - Negative < -0.03%: Shorts paying longs. Market overheated short. Contrarian bullish.
  - Near 0%: Neutral, follow momentum.
- `web_search("bitcoin open interest change today")` — positioning
  - OI rising + price rising = new longs entering (bullish continuation)
  - OI rising + price falling = new shorts entering (bearish continuation)
  - OI falling + price rising = short squeeze (explosive but may reverse)
  - OI falling + price falling = long liquidation (may bounce after washout)

### E. BTC Dominance & Rotation
- `web_search("bitcoin dominance today")` — BTC.D percentage
  - BTC.D rising = money flowing TO BTC FROM alts (trade BTC, avoid ETH)
  - BTC.D falling = money flowing FROM BTC TO alts (ETH may outperform)
  - BTC.D stable = broad crypto rally or broad crypto sell-off

### F. Macro Context
- `web_search("S&P 500 today")` — BTC correlates with risk assets
- `web_search("DXY dollar index today")` — strong dollar = headwind for crypto
- `get_positions()` — existing positions
- `get_balance()` — available margin
- `get_risk_status()` — daily PnL and limits

---

## STEP 2: Determine Regime

Before scoring, classify the current REGIME. This changes how you interpret signals.

### Regime Classification

**TRENDING UP (Bull)** — BTC above 20-day high, making higher highs
- Strategy: Buy dips, ride momentum, use trailing stops
- Funding rate signal: IGNORE moderate positive funding (it is normal in uptrends)
- Fear/Greed: readings of 60-75 are NORMAL, not a sell signal

**TRENDING DOWN (Bear)** — BTC below 20-day low, making lower lows
- Strategy: Sell rallies or stay flat. Short only with extreme conviction.
- Funding rate signal: negative funding is expected, not a buy signal
- Fear/Greed: readings of 25-40 are NORMAL, not a buy signal

**RANGE-BOUND** — BTC chopping between support and resistance
- Strategy: Fade extremes. Buy at range bottom, sell at range top.
- Best regime for contrarian signals (funding, fear/greed)
- Use tighter stops (range will break eventually)

**BREAKOUT** — BTC just broke out of a multi-week range with volume
- Strategy: Trade the breakout direction. Do NOT fade.
- Volume surge + breakout = real move, not a fake-out
- Enter on first pullback to breakout level (retest)

Determine the regime from the price data gathered in Step 1.

---

## STEP 3: Score Momentum Signals

Score each factor from -2 to +2 (positive = bullish, negative = bearish):

**A. Price Momentum (weight: 2.0x)**
- Breaking out to new 30-day high with volume → +2
- Breaking out to new 7-day high → +1
- Ranging, no clear direction → 0
- Breaking down to new 7-day low → -1
- Breaking down to new 30-day low with volume → -2

**B. Funding Rate Signal (weight: 1.5x)**
Read CONTRARIAN to funding in range-bound markets, WITH funding in trending markets.

In RANGE-BOUND regime:
- Funding > 0.1%: overcrowded longs → score -2 (fade)
- Funding 0.05-0.1%: elevated longs → score -1
- Funding -0.03% to 0.05%: neutral → score 0
- Funding -0.1% to -0.03%: elevated shorts → score +1
- Funding < -0.1%: overcrowded shorts → score +2 (fade)

In TRENDING regime:
- Funding aligns with trend direction: score 0 (normal)
- Funding OPPOSES trend at extreme (e.g., negative funding in uptrend): score +2 (fuel for continuation)
- Funding same direction as trend at extreme: score -1 (overheated, pullback likely)

**C. ETF / Institutional Flows (weight: 1.5x)** — BTC only
- Net inflows > $500M in last 3 days → +2
- Net inflows > $100M → +1
- Flat or mixed → 0
- Net outflows > $100M → -1
- Net outflows > $500M → -2

For ETH: use exchange flow data as proxy (outflows = bullish, inflows = bearish).

**D. Fear/Greed Index (weight: 1.0x)**
Read CONTRARIAN:
- Extreme Fear (0-20) → score +2 (buy when others panic)
- Fear (20-35) → score +1
- Neutral (35-65) → score 0
- Greed (65-80) → score -1
- Extreme Greed (80-100) → score -2 (sell when others are euphoric)

**E. On-Chain / Whale Activity (weight: 1.0x)**
- Large exchange outflows + whale accumulation → +2
- Moderate outflows → +1
- Neutral → 0
- Moderate exchange inflows → -1
- Large exchange inflows + whale deposits → -2

**F. Macro Correlation (weight: 0.5x)**
- S&P 500 rallying + DXY falling → +2 (risk-on, crypto-friendly)
- S&P stable → 0
- S&P falling + DXY rising → -2 (risk-off, crypto-hostile)

### Compute Weighted Score

```
weighted_score = (A*2.0 + B*1.5 + C*1.5 + D*1.0 + E*1.0 + F*0.5) / 7.5
```

---

## STEP 4: BTC Dominance Filter (for ETH trades)

Before trading ETHUSDT, check BTC dominance:

| BTC.D Trend       | ETH Trade Rule                                    |
|--------------------|---------------------------------------------------|
| Rising > 1% / week | DO NOT buy ETH. Money is flowing to BTC.          |
| Stable             | ETH trades allowed, follow momentum signals.       |
| Falling > 1% / week| ETH may outperform. Prefer ETH over BTC for longs.|

If BTC.D is rising sharply and you want to go long crypto, trade BTC, not ETH.

---

## STEP 5: Signal Generation

| Weighted Score     | Signal        | Action                                |
|--------------------|---------------|---------------------------------------|
| >= +1.0            | STRONG BUY    | Full position size                    |
| +0.5 to +0.99     | BUY           | Half position size                    |
| -0.49 to +0.49    | HOLD          | No new positions                      |
| -0.99 to -0.5     | SELL          | Half position size short              |
| <= -1.0           | STRONG SELL   | Full position size short              |

### Regime Override
- In TRENDING UP regime: lower buy threshold to +0.3 (momentum carries)
- In TRENDING DOWN regime: lower sell threshold to -0.3
- In BREAKOUT regime: follow breakout direction if score is > 0 (for up) or < 0 (for down)
- In RANGE-BOUND: keep thresholds at +/-0.5 (need stronger signal to fade range)

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
- 3+ signals aligned (momentum + funding + flows): +5
- Breakout with volume confirmation: +5
- Post-liquidation cascade (washout): +5 (for contrarian entry)
- Conflicting signals (momentum up but funding extreme): -10
- Weekend (Saturday/Sunday, lower liquidity): -5
- Extreme greed > 90 for a long trade: -10
- Extreme fear < 10 for a short trade: -10
- BTC.D opposing your ETH trade: -10

---

## STEP 7: Position Sizing

Crypto is 3-5x more volatile than forex. Size accordingly.

| Confidence | BTC Size | ETH Size |
|------------|----------|----------|
| 90-95      | 0.002    | 0.02     |
| 85-89      | 0.0015   | 0.015    |
| 80-84      | 0.001    | 0.01     |
| 75-79      | 0.001    | 0.01     |
| 70-74      | 0.0005   | 0.005    |
| < 70       | 0 (HOLD) | 0 (HOLD) |

These sizes are for a ~$500-$1000 account. Adjust proportionally.

### Hard Limits
- Maximum 1 BTC position + 1 ETH position at any time
- Maximum total crypto exposure: 3% of equity per position
- If daily PnL loss > 3% of equity from crypto: STOP trading crypto for 24 hours
- NEVER go all-in. Crypto can drop 15% in an hour.

---

## STEP 8: Stop Loss and Take Profit

### BTC Stop Loss Rules
| Regime       | SL Distance          | TP Distance          | R:R    |
|--------------|----------------------|----------------------|--------|
| Trending     | 3-5% from entry      | 8-15% from entry     | 2.5-3x |
| Range-bound  | 2-3% from entry      | 4-8% from entry      | 2x     |
| Breakout     | Below breakout level | 6-12% from entry     | 2-3x   |

### ETH Stop Loss Rules
ETH is more volatile than BTC. Widen stops by 1.5x.
| Regime       | SL Distance          | TP Distance          | R:R    |
|--------------|----------------------|----------------------|--------|
| Trending     | 5-7% from entry      | 12-20% from entry    | 2.5-3x |
| Range-bound  | 3-5% from entry      | 6-12% from entry     | 2x     |
| Breakout     | Below breakout level | 8-16% from entry     | 2-3x   |

### Trailing Stop Rules
- After position reaches 1.5x SL distance in profit:
  - Move stop to breakeven
- After position reaches 2x SL distance in profit:
  - Trail stop at 50% of unrealized profit
- In a strong trend: widen trail to capture extended moves

### Hard Floor
- BTC: SL never tighter than 2% from entry. Anything less gets whipsawed.
- ETH: SL never tighter than 3% from entry.

---

## STEP 9: Execute

1. Call `log_decision` with:
   - symbol: "BTCUSDT" or "ETHUSDT"
   - action: "buy" / "sell" / "hold"
   - confidence: computed value
   - reasoning: Include regime, all 6 signal scores, weighted score, BTC.D check (for ETH),
     and the specific data points driving the decision
   - sources: search queries and key findings

2. If confidence >= 70 AND no conflicting filters:
   - Call `trade(action, symbol, size)`

3. Otherwise:
   - Log as HOLD with full reasoning
   - Do NOT call trade

---

## STEP 10: Liquidation Cascade Detection

Liquidation cascades create the best contrarian entry opportunities in crypto.

### Signs of a Liquidation Cascade
- Price drops 8%+ in under 2 hours
- Funding rate goes deeply negative (< -0.1%)
- Open interest drops sharply (liquidations clearing positions)
- Fear/Greed drops below 15
- Social media is in full panic mode

### How to Trade the Cascade
1. DO NOT buy during the cascade. Wait for the flush to complete.
2. Look for: price stabilization for 30+ minutes after the dump
3. Look for: funding rate starting to normalize (moving back toward 0)
4. Look for: exchange outflows resuming (smart money accumulating)
5. Enter with half position size. Add the other half if it holds for 4 hours.
6. SL: below the cascade low. TP: 50-80% retracement of the dump.

### Signs of a Melt-Up / Blow-Off Top
- Price rises 15%+ in under 24 hours
- Funding rate > 0.15% (longs paying extreme premium)
- Fear/Greed > 90
- Social media is euphoric, mainstream media covering crypto
- Google Trends for "bitcoin" spiking

### How to Trade the Blow-Off
1. DO NOT short during the melt-up. Wait for the top to form.
2. Look for: first major red candle (4H) after the spike
3. Look for: funding rate starting to drop from extreme
4. Look for: exchange inflows spiking (whales depositing to sell)
5. Enter short with half position size on the first pullback.
6. SL: above the blow-off high. TP: 30-50% retracement of the melt-up.

---

## STEP 11: BTC/ETH Relative Value

Sometimes the best trade is the RATIO, not the direction.

### When to Prefer BTC over ETH (for longs)
- BTC.D rising
- Risk-off in traditional markets (BTC = "digital gold")
- ETH-specific negative news (regulatory, hack)
- Early cycle (BTC leads, ETH follows)

### When to Prefer ETH over BTC (for longs)
- BTC.D falling
- Risk-on + DeFi narrative strong
- ETH upgrade/catalyst approaching
- Late cycle (ETH outperforms in blow-off)

If both signals are equal, default to BTC (more liquid, tighter spreads).

---

## Time-of-Day Considerations

Crypto trades 24/7 but liquidity varies:

| Window (UTC)   | Liquidity | Notes                                          |
|----------------|-----------|------------------------------------------------|
| 08:00-16:00    | HIGH      | EU + US overlap. Best execution.               |
| 13:00-21:00    | HIGH      | US session. Most volume.                       |
| 00:00-08:00    | MEDIUM    | Asian session. Decent for BTC.                 |
| 21:00-00:00    | LOW       | Dead zone. Wider spreads, thin books.          |
| Sat-Sun        | LOW       | Weekend. Liquidity drops 40-60%.               |

- Weekend: reduce position size by 50% OR avoid trading entirely
- "Dead zone" (21:00-00:00 UTC): avoid entering new positions

---

## Anti-Patterns (NEVER DO THESE)

1. **Do NOT buy during a liquidation cascade**. Wait for the flush to complete.
2. **Do NOT short during a melt-up**. You will get squeezed.
3. **Do NOT ignore funding rates**. Extreme funding always corrects.
4. **Do NOT trade ETH when BTC.D is surging**. Money is leaving alts.
5. **Do NOT use forex-sized stops on crypto**. 2% SL on BTC = instant stop-out on noise.
6. **Do NOT hold leveraged positions over the weekend**. Liquidity gaps are real.
7. **Do NOT chase a 10%+ move**. The easy money is already made.
8. **Do NOT trade altcoins other than ETH**. Insufficient liquidity and too many rug-pulls.
9. **Do NOT fight the macro**. If S&P is crashing and DXY is surging, crypto will bleed.
10. **Do NOT believe "this time is different"**. Crypto cycles repeat with reliable regularity.
11. **Do NOT trade based solely on social media hype**. It is a lagging indicator.
12. **Do NOT average down** in crypto. If BTC drops 10%, it can easily drop 30%.

---

## Example Reasoning (for log_decision)

```
"BTCUSDT momentum analysis:
Regime: TRENDING UP — BTC at $72,400, above 30-day high, making higher highs since $65K.
Price Momentum: +2 (new monthly high, strong volume on breakout above $70K).
Funding Rate: -1 (0.08% — elevated in trending market, slight overheating).
ETF Flows: +2 ($850M net inflows last 3 days — institutional demand strong).
Fear/Greed: -1 (78 — Greed zone, but normal for uptrend. Not extreme enough to fade).
On-Chain: +1 (exchange outflows continuing, whale wallets accumulating).
Macro: +1 (S&P at highs, DXY falling — risk-on environment).
Weighted: (2*2.0 + (-1)*1.5 + 2*1.5 + (-1)*1.0 + 1*1.0 + 1*0.5) / 7.5 = +0.67.
Regime override: trending up, threshold lowered to +0.3. Signal: BUY.
Confidence: base 75 + 5 (3 signals aligned) = 80.
BTC.D: 54.2%, rising — confirms BTC over ETH preference.
Session: US session (15:30 UTC) — high liquidity.
Position: 0.001 BTC. SL: $69,800 (3.6% below entry). TP: $78,500 (8.4% above).
R:R = 2.3:1. Acceptable.
Decision: BUY BTCUSDT 0.001 at market."
```
