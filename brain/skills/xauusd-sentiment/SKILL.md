---
name: xauusd-sentiment
description: "Trade XAUUSD based on news sentiment and macro analysis. Use when: user wants gold trading, sentiment-driven approach, or broader macro view beyond just calendar events."
---

# XAUUSD Sentiment Trading

## Strategy
Aggregate sentiment from multiple sources to determine gold's directional bias.

## Data sources to check
1. **Gold news**: Search "gold price today" — check Reuters, Bloomberg, Kitco
2. **Fed commentary**: Search "Federal Reserve statement" — hawkish vs dovish
3. **Geopolitics**: Search "geopolitical risk gold" — wars, sanctions, elections
4. **DXY (Dollar Index)**: Search "dollar index today" — inverse correlation
5. **US Treasury yields**: Search "US 10 year yield" — inverse correlation

## Scoring system
Assign each factor a score from -2 (very bearish) to +2 (very bullish):
- Fed policy outlook: _____
- USD strength (DXY): _____ (inverted — strong USD = negative)
- Geopolitical risk: _____ (more risk = positive for gold)
- Market sentiment: _____
- Technical trend: _____

Total score: sum / 5 factors
- Score > 0.5: BUY signal
- Score < -0.5: SELL signal
- Score between -0.5 and 0.5: HOLD

## Confidence mapping
- |score| >= 1.5: confidence 90
- |score| >= 1.0: confidence 80
- |score| >= 0.5: confidence 70
- |score| < 0.5: confidence < 70 → HOLD (don't trade)

## Position sizing
- confidence >= 90: 0.02 lots
- confidence >= 80: 0.01 lots
- confidence >= 70: 0.01 lots
