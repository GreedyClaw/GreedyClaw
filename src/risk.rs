use crate::config::RiskConfig;
use crate::error::AppError;
use crate::exchange::types::*;

use chrono::{Datelike, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;
use tracing::{info, warn};

/// Tracked position for mark-to-market PnL.
#[derive(Debug, Clone)]
struct TrackedPosition {
    symbol: String,
    quantity: f64,
    avg_entry_price: f64,
    mark_price: f64, // last known price for floating PnL
    #[allow(dead_code)]
    side: OrderSide,
}

/// Rate limiter state: sliding window of trade timestamps.
struct RateLimiter {
    timestamps: Mutex<Vec<i64>>,
    max_per_minute: u32,
}

impl RateLimiter {
    fn new(max_per_minute: u32) -> Self {
        Self {
            timestamps: Mutex::new(Vec::new()),
            max_per_minute,
        }
    }

    /// Check if a new trade is allowed. Returns Err if rate exceeded.
    fn check(&self) -> Result<(), AppError> {
        let now = Utc::now().timestamp_millis();
        let cutoff = now - 60_000; // 1 minute window

        let mut ts = self.timestamps.lock().unwrap();
        ts.retain(|&t| t > cutoff);

        if ts.len() >= self.max_per_minute as usize {
            warn!(
                "[RISK] Rate limit: {} trades in last 60s (max: {})",
                ts.len(),
                self.max_per_minute
            );
            return Err(AppError::RateLimit(format!(
                "{} trades in last 60s exceeds limit of {}. Possible hallucination loop.",
                ts.len(),
                self.max_per_minute
            )));
        }

        ts.push(now);
        Ok(())
    }
}

/// Risk snapshot included in every trade response.
#[derive(Debug, Clone, Serialize)]
pub struct RiskSnapshot {
    pub open_positions: usize,
    pub max_open_positions: usize,
    pub realized_daily_pnl: f64,
    pub floating_pnl: f64,
    pub total_daily_pnl: f64,
    pub remaining_daily_limit: f64,
    pub trades_last_minute: u32,
    pub max_trades_per_minute: u32,
}

pub struct RiskEngine {
    config: RiskConfig,
    positions: DashMap<String, TrackedPosition>,
    /// Realized daily PnL in USD (stored as cents to use atomic)
    daily_pnl_cents: AtomicI32,
    daily_trades: AtomicU32,
    /// Day-of-year for daily reset
    pnl_day: AtomicU32,
    rate_limiter: RateLimiter,
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        let today = Utc::now().ordinal();
        Self {
            rate_limiter: RateLimiter::new(config.max_trades_per_minute),
            config,
            positions: DashMap::new(),
            daily_pnl_cents: AtomicI32::new(0),
            daily_trades: AtomicU32::new(0),
            pnl_day: AtomicU32::new(today),
        }
    }

    /// Reset daily counters if day changed.
    fn maybe_reset_daily(&self) {
        let today = Utc::now().ordinal();
        let stored = self.pnl_day.load(Ordering::Relaxed);
        if today != stored
            && self
                .pnl_day
                .compare_exchange(stored, today, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
        {
            self.daily_pnl_cents.store(0, Ordering::Relaxed);
            self.daily_trades.store(0, Ordering::Relaxed);
            info!("[RISK] Daily counters reset (new day)");
        }
    }

    /// Pre-trade validation. Called BEFORE every exchange call.
    pub fn check_pre_trade(
        &self,
        symbol: &str,
        side: OrderSide,
        quantity: f64,
        price_usd: f64,
    ) -> Result<(), AppError> {
        self.maybe_reset_daily();

        // 1. Rate limit (circuit breaker)
        self.rate_limiter.check()?;

        // 2. Symbol whitelist
        if !self.config.allowed_symbols.is_empty()
            && !self.config.allowed_symbols.iter().any(|s| s == symbol)
        {
            return Err(AppError::RiskViolation(format!(
                "Symbol {symbol} not in allowed list: {:?}",
                self.config.allowed_symbols
            )));
        }

        // 3. Quantity & price validation (reject NaN, Infinity, negatives)
        if !quantity.is_finite() || quantity <= 0.0 {
            return Err(AppError::Validation("Quantity must be a finite positive number".into()));
        }
        if !price_usd.is_finite() || price_usd <= 0.0 {
            return Err(AppError::Validation("Price must be a finite positive number".into()));
        }
        // Sanity cap — no single trade over $10M (prevents f64 overflow exploits)
        if quantity > 1e9 || price_usd > 1e9 {
            return Err(AppError::Validation("Amount or price exceeds sanity limit".into()));
        }

        // 3b. Update mark price for the symbol (used for floating PnL)
        if let Some(mut pos) = self.positions.get_mut(symbol) {
            pos.mark_price = price_usd;
        }

        // 4. Position size limit
        let exposure_usd = quantity * price_usd;
        if exposure_usd > self.config.max_position_usd {
            return Err(AppError::RiskViolation(format!(
                "Position ${:.2} exceeds max ${:.2}",
                exposure_usd, self.config.max_position_usd
            )));
        }

        // 5. Max open positions (only for new buys)
        if side == OrderSide::Buy
            && !self.positions.contains_key(symbol)
            && self.positions.len() >= self.config.max_open_positions
        {
            return Err(AppError::RiskViolation(format!(
                "{} open positions, max is {}",
                self.positions.len(),
                self.config.max_open_positions
            )));
        }

        // 6. Daily loss limit (realized + floating)
        let realized = self.daily_pnl_cents.load(Ordering::Relaxed) as f64 / 100.0;
        let floating = self.floating_pnl();
        let total_pnl = realized + floating;
        if total_pnl < -self.config.max_daily_loss_usd {
            return Err(AppError::RiskViolation(format!(
                "Daily PnL ${:.2} (realized ${:.2} + floating ${:.2}) exceeds max loss -${:.2}",
                total_pnl, realized, floating, self.config.max_daily_loss_usd
            )));
        }

        info!(
            "[RISK] Pre-trade OK: {} {} {} @ ${:.2} (exposure: ${:.2}, daily PnL: ${:.2})",
            side, quantity, symbol, price_usd, exposure_usd, total_pnl
        );

        Ok(())
    }

    /// Record a fill. Updates position tracking and daily PnL.
    pub fn record_fill(&self, result: &OrderResult) {
        self.daily_trades.fetch_add(1, Ordering::Relaxed);

        let symbol = &result.symbol;

        match result.side {
            OrderSide::Buy => {
                // Open or add to position
                self.positions
                    .entry(symbol.clone())
                    .and_modify(|pos| {
                        let total_qty = pos.quantity + result.filled_qty;
                        pos.avg_entry_price = (pos.avg_entry_price * pos.quantity
                            + result.avg_price * result.filled_qty)
                            / total_qty;
                        pos.quantity = total_qty;
                    })
                    .or_insert(TrackedPosition {
                        symbol: symbol.clone(),
                        quantity: result.filled_qty,
                        avg_entry_price: result.avg_price,
                        mark_price: result.avg_price,
                        side: OrderSide::Buy,
                    });
            }
            OrderSide::Sell => {
                // Close or reduce position — calculate realized PnL
                if let Some(mut pos) = self.positions.get_mut(symbol) {
                    let pnl = (result.avg_price - pos.avg_entry_price) * result.filled_qty;
                    let pnl_cents = (pnl * 100.0) as i32;
                    self.daily_pnl_cents.fetch_add(pnl_cents, Ordering::Relaxed);

                    pos.quantity -= result.filled_qty;
                    if pos.quantity <= 0.001 {
                        // Position effectively closed
                        drop(pos);
                        self.positions.remove(symbol);
                    }

                    info!(
                        "[RISK] Realized PnL on {}: ${:.2} (daily total: ${:.2})",
                        symbol,
                        pnl,
                        self.daily_pnl_cents.load(Ordering::Relaxed) as f64 / 100.0
                    );
                }
            }
        }
    }

    /// Calculate floating PnL across all positions using last known mark prices.
    fn floating_pnl(&self) -> f64 {
        self.positions
            .iter()
            .map(|entry| {
                let pos = entry.value();
                if pos.mark_price > 0.0 {
                    (pos.mark_price - pos.avg_entry_price) * pos.quantity
                } else {
                    0.0 // no mark price yet — conservative assumption
                }
            })
            .sum()
    }

    /// Update mark-to-market price for a symbol.
    /// Called during pre-trade check and can be called periodically for live PnL.
    #[allow(dead_code)]
    pub fn update_mark_price(&self, symbol: &str, current_price: f64) -> f64 {
        if let Some(mut pos) = self.positions.get_mut(symbol) {
            pos.mark_price = current_price;
            (current_price - pos.avg_entry_price) * pos.quantity
        } else {
            0.0
        }
    }

    /// Get current risk snapshot for API response.
    pub fn snapshot(&self) -> RiskSnapshot {
        self.maybe_reset_daily();
        let realized = self.daily_pnl_cents.load(Ordering::Relaxed) as f64 / 100.0;
        let floating = self.floating_pnl();
        let total = realized + floating;

        let trades_last_min = self
            .rate_limiter
            .timestamps
            .lock()
            .map(|ts| {
                let cutoff = Utc::now().timestamp_millis() - 60_000;
                ts.iter().filter(|&&t| t > cutoff).count() as u32
            })
            .unwrap_or(0);

        RiskSnapshot {
            open_positions: self.positions.len(),
            max_open_positions: self.config.max_open_positions,
            realized_daily_pnl: realized,
            floating_pnl: floating,
            total_daily_pnl: total,
            remaining_daily_limit: self.config.max_daily_loss_usd + total,
            trades_last_minute: trades_last_min,
            max_trades_per_minute: self.config.max_trades_per_minute,
        }
    }

    /// Force-reset daily counters (for testing).
    #[cfg(test)]
    pub fn force_reset_daily(&self) {
        self.daily_pnl_cents.store(0, Ordering::Relaxed);
        self.daily_trades.store(0, Ordering::Relaxed);
    }

    /// Get all tracked positions (for GET /positions).
    pub fn get_positions(&self) -> Vec<Position> {
        self.positions
            .iter()
            .map(|entry| {
                let pos = entry.value();
                let unrealized = if pos.mark_price > 0.0 {
                    (pos.mark_price - pos.avg_entry_price) * pos.quantity
                } else {
                    0.0
                };
                Position {
                    symbol: pos.symbol.clone(),
                    quantity: pos.quantity,
                    avg_entry_price: pos.avg_entry_price,
                    current_price: pos.mark_price,
                    unrealized_pnl: unrealized,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RiskConfig;
    use crate::exchange::types::OrderSide;

    fn test_config() -> RiskConfig {
        RiskConfig {
            max_position_usd: 1000.0,
            max_daily_loss_usd: 100.0,
            max_open_positions: 2,
            allowed_symbols: vec!["BTCUSDT".into(), "ETHUSDT".into()],
            max_trades_per_minute: 3,
        }
    }

    #[test]
    fn test_rate_limiter() {
        let engine = RiskEngine::new(test_config());
        // 3 trades should succeed (max_trades_per_minute = 3)
        for _ in 0..3 {
            engine
                .check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, 50000.0)
                .unwrap();
        }
        // 4th trade should trigger circuit breaker
        let result = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, 50000.0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::RateLimit(_)),
            "Expected RateLimit, got: {:?}",
            err
        );
    }

    #[test]
    fn test_position_limit() {
        // max_open_positions = 1, empty whitelist (allow all)
        let engine = RiskEngine::new(RiskConfig {
            max_open_positions: 1,
            allowed_symbols: vec![],
            max_trades_per_minute: 100,
            ..test_config()
        });

        // Open first position
        engine
            .check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, 50000.0)
            .unwrap();
        engine.record_fill(&OrderResult {
            exchange_order_id: "1".into(),
            client_order_id: "c1".into(),
            symbol: "BTCUSDT".into(),
            side: OrderSide::Buy,
            filled_qty: 0.01,
            avg_price: 50000.0,
            status: OrderStatus::Filled,
            timestamp: Utc::now(),
            commission: 0.0,
        });

        // Second position on a different symbol should be rejected
        let result = engine.check_pre_trade("ETHUSDT", OrderSide::Buy, 0.1, 3000.0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::RiskViolation(_)));
    }

    #[test]
    fn test_daily_loss_limit() {
        let engine = RiskEngine::new(RiskConfig {
            max_daily_loss_usd: 50.0,
            allowed_symbols: vec![],
            max_trades_per_minute: 100,
            ..test_config()
        });

        // Simulate a big realized loss: buy high, sell low
        engine.record_fill(&OrderResult {
            exchange_order_id: "1".into(),
            client_order_id: "c1".into(),
            symbol: "BTCUSDT".into(),
            side: OrderSide::Buy,
            filled_qty: 1.0,
            avg_price: 100.0,
            status: OrderStatus::Filled,
            timestamp: Utc::now(),
            commission: 0.0,
        });
        // Sell at $40 -> realized PnL = (40 - 100) * 1.0 = -$60
        engine.record_fill(&OrderResult {
            exchange_order_id: "2".into(),
            client_order_id: "c2".into(),
            symbol: "BTCUSDT".into(),
            side: OrderSide::Sell,
            filled_qty: 1.0,
            avg_price: 40.0,
            status: OrderStatus::Filled,
            timestamp: Utc::now(),
            commission: 0.0,
        });

        // Daily PnL should now be -$60, exceeding max_daily_loss of $50
        let result = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, 100.0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::RiskViolation(_)));
    }

    #[test]
    fn test_nan_rejection() {
        let engine = RiskEngine::new(RiskConfig {
            allowed_symbols: vec![],
            max_trades_per_minute: 100,
            ..test_config()
        });

        // NaN price
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, f64::NAN);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // Infinity quantity
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, f64::INFINITY, 100.0);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // NaN quantity
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, f64::NAN, 100.0);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // Infinity price
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, f64::INFINITY);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_symbol_whitelist() {
        let engine = RiskEngine::new(test_config());

        // Allowed symbol should pass
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.01, 50000.0);
        assert!(r.is_ok());

        // Non-whitelisted symbol should be rejected
        let r = engine.check_pre_trade("DOGEUSDT", OrderSide::Buy, 100.0, 0.1);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::RiskViolation(_)));
    }

    #[test]
    fn test_quantity_sanity() {
        let engine = RiskEngine::new(RiskConfig {
            allowed_symbols: vec![],
            max_trades_per_minute: 100,
            ..test_config()
        });

        // Negative quantity
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, -1.0, 100.0);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // Zero quantity
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 0.0, 100.0);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // Huge quantity (> 1e9 sanity cap)
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 2e9, 100.0);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));

        // Huge price (> 1e9 sanity cap)
        let r = engine.check_pre_trade("BTCUSDT", OrderSide::Buy, 1.0, 2e9);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_daily_reset() {
        let engine = RiskEngine::new(RiskConfig {
            allowed_symbols: vec![],
            max_trades_per_minute: 100,
            ..test_config()
        });

        // Simulate some daily PnL
        engine.record_fill(&OrderResult {
            exchange_order_id: "1".into(),
            client_order_id: "c1".into(),
            symbol: "BTCUSDT".into(),
            side: OrderSide::Buy,
            filled_qty: 1.0,
            avg_price: 100.0,
            status: OrderStatus::Filled,
            timestamp: Utc::now(),
            commission: 0.0,
        });
        engine.record_fill(&OrderResult {
            exchange_order_id: "2".into(),
            client_order_id: "c2".into(),
            symbol: "BTCUSDT".into(),
            side: OrderSide::Sell,
            filled_qty: 1.0,
            avg_price: 110.0,
            status: OrderStatus::Filled,
            timestamp: Utc::now(),
            commission: 0.0,
        });

        // Verify non-zero state
        let snap = engine.snapshot();
        assert!(snap.realized_daily_pnl != 0.0);

        // Force reset
        engine.force_reset_daily();

        let snap = engine.snapshot();
        assert_eq!(snap.realized_daily_pnl, 0.0);
    }
}
