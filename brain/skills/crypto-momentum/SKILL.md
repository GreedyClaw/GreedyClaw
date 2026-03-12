---
name: crypto-momentum
description: "Trade crypto based on momentum and social sentiment. Use when: market is crypto, symbols include BTC/ETH, or social media hype is detected."
---

# Crypto Momentum Trading

## Strategy
Trade BTC/ETH based on social momentum, on-chain metrics, and macro sentiment.

## Data sources
1. **Crypto news**: Search "bitcoin news today" — regulatory, ETF flows, whale movements
2. **Social sentiment**: Search "bitcoin twitter sentiment" or "crypto fear greed index"
3. **On-chain**: Search "bitcoin exchange inflows" — large inflows = sell pressure
4. **Funding rates**: Check via get_price — elevated funding = overheated longs
5. **Macro correlation**: Search "S&P 500 today" — BTC correlates with risk assets

## Decision framework
- **ETF inflows positive + Fear/Greed < 30**: Strong BUY (contrarian)
- **ETF outflows + Fear/Greed > 80**: Strong SELL (contrarian)
- **Breaking news (regulation, hack, ban)**: React to direction
- **Whale accumulation + low funding**: BUY
- **Exchange inflows spike + high funding**: SELL

## Risk rules
- BTC position max: 0.001 BTC
- ETH position max: 0.01 ETH
- Never trade during weekends (low liquidity)
- Max 2 trades per day
