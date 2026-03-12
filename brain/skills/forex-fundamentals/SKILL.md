---
name: forex-fundamentals
description: "Trade forex/gold based on economic calendar events. Use when: market is forex, symbols include XAUUSD/EURUSD/GBPUSD, or high-impact economic events are happening (NFP, CPI, FOMC, ECB)."
---

# Forex Fundamentals Trading

## Strategy
Trade gold and major pairs around high-impact economic events.

## Process
1. Check economic calendar (Forex Factory) for today's HIGH IMPACT events
2. Analyze the event's likely impact on USD and gold:
   - **NFP/Jobs**: Strong NFP → USD up → Gold down. Weak → opposite.
   - **CPI/Inflation**: High CPI → Fed hawkish → USD up → Gold down.
   - **FOMC/Fed**: Hawkish tone → USD up. Dovish → Gold up.
   - **GDP**: Strong GDP → USD up. Weak → Gold up (safe haven).
3. Check if event has already been released (actual vs forecast):
   - If actual > forecast: surprise positive for the currency
   - If actual < forecast: surprise negative
4. Trade the deviation (actual - forecast), not the direction alone

## Rules
- Only trade around HIGH IMPACT events
- Wait for actual data release before trading (no guessing)
- Minimum deviation threshold: actual must differ from forecast
- Gold inverse to USD: USD strength = Gold weakness
- Session hours: 7:00-20:00 UTC only (Asian session spreads are too wide)
- Position size: 0.01 lots for XAUUSD (minimum risk)

## Key relationships
- Gold (XAUUSD) = anti-dollar, safe haven
- EURUSD = inversely correlated with USD strength
- DXY (Dollar Index) rising = bearish for Gold and EURUSD
- US 10Y yield rising = bearish for Gold
- Geopolitical risk = bullish for Gold

## Risk management
- SL must be >= 1.0 ATR to survive spread ($0.50-0.80 on gold)
- Never trade during first 30 seconds after release (spike + retrace)
- Max 1 trade per event
- If no high-impact events today: HOLD
