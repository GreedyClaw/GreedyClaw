/// Strategy engine — manages auto-trading when scanner triggers fire.
/// Tracks positions opened by the scanner and handles exit logic.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use dashmap::DashMap;
use tracing::{info, warn};

use super::scoring::TriggerSignal;

/// Exit configuration for scanner positions.
#[derive(Debug, Clone)]
pub struct ExitConfig {
    pub sl_pct: f64,
    pub tp_pct: f64,
    pub trail_activation_pct: f64,
    pub trail_dd_pct: f64,
    pub timeout_s: f64,
}

impl Default for ExitConfig {
    fn default() -> Self {
        Self {
            sl_pct: -15.0,
            tp_pct: 120.0,
            trail_activation_pct: 26.0,
            trail_dd_pct: -8.0,
            timeout_s: 165.0,
        }
    }
}

/// A position opened by the scanner.
pub struct ScannerPosition {
    pub mint: String,
    pub strategy: String,
    pub entry_price_sol: f64,
    pub entry_time: Instant,
    pub peak_price_sol: f64,
    pub current_price_sol: f64,
    pub sol_invested: f64,
    pub exit_config: ExitConfig,
    pub exited: bool,
}

impl ScannerPosition {
    pub fn return_pct(&self) -> f64 {
        if self.entry_price_sol <= 0.0 { return 0.0; }
        (self.current_price_sol / self.entry_price_sol - 1.0) * 100.0
    }

    pub fn peak_return_pct(&self) -> f64 {
        if self.entry_price_sol <= 0.0 { return 0.0; }
        (self.peak_price_sol / self.entry_price_sol - 1.0) * 100.0
    }

    pub fn elapsed_s(&self) -> f64 {
        self.entry_time.elapsed().as_secs_f64()
    }

    pub fn unrealized_pnl_sol(&self) -> f64 {
        self.sol_invested * self.return_pct() / 100.0
    }
}

/// Serializable position info for API.
#[derive(Debug, Clone, Serialize)]
pub struct PositionInfo {
    pub mint: String,
    pub strategy: String,
    pub entry_price_sol: f64,
    pub current_price_sol: f64,
    pub return_pct: f64,
    pub peak_return_pct: f64,
    pub unrealized_pnl_sol: f64,
    pub elapsed_s: f64,
    pub sol_invested: f64,
}

/// Check exit condition. Returns reason or None.
pub fn check_exit(pos: &ScannerPosition) -> Option<&'static str> {
    let ret = pos.return_pct();
    let peak_ret = pos.peak_return_pct();
    let elapsed = pos.elapsed_s();
    let cfg = &pos.exit_config;

    // 1. Stop loss
    if ret <= cfg.sl_pct {
        return Some("STOP_LOSS");
    }

    // 2. Take profit
    if ret >= cfg.tp_pct {
        return Some("TAKE_PROFIT");
    }

    // 3. Trailing stop
    if peak_ret >= cfg.trail_activation_pct {
        let trail_trigger = peak_ret + cfg.trail_dd_pct;
        if ret <= trail_trigger {
            return Some("TRAILING_STOP");
        }
    }

    // 4. Timeout
    if elapsed >= cfg.timeout_s {
        return Some("TIMEOUT");
    }

    None
}

/// Strategy manager — tracks scanner positions.
pub struct StrategyManager {
    pub positions: DashMap<String, ScannerPosition>,
    pub total_triggers: AtomicU64,
    pub total_trades: AtomicU64,
    pub wins: AtomicU64,
    pub losses: AtomicU64,
}

impl StrategyManager {
    pub fn new() -> Self {
        Self {
            positions: DashMap::new(),
            total_triggers: AtomicU64::new(0),
            total_trades: AtomicU64::new(0),
            wins: AtomicU64::new(0),
            losses: AtomicU64::new(0),
        }
    }

    /// Record a new trigger (may or may not result in a trade).
    pub fn on_trigger(&self, signal: &TriggerSignal, entry_sol: f64, max_positions: usize) -> bool {
        self.total_triggers.fetch_add(1, Ordering::Relaxed);

        if self.positions.contains_key(&signal.mint) {
            return false;
        }

        if self.positions.len() >= max_positions {
            warn!(
                "[STRATEGY] Capacity full ({}/{}), skipping {}",
                self.positions.len(), max_positions, &signal.mint[..signal.mint.len().min(12)]
            );
            return false;
        }

        self.positions.insert(signal.mint.clone(), ScannerPosition {
            mint: signal.mint.clone(),
            strategy: signal.strategy.clone(),
            entry_price_sol: signal.price_sol,
            entry_time: Instant::now(),
            peak_price_sol: signal.price_sol,
            current_price_sol: signal.price_sol,
            sol_invested: entry_sol,
            exit_config: ExitConfig::default(),
            exited: false,
        });

        info!(
            "[STRATEGY] ENTER {} | {} | price={:.12} SOL | budget={:.3} SOL",
            &signal.mint[..signal.mint.len().min(12)],
            signal.strategy,
            signal.price_sol,
            entry_sol,
        );

        true
    }

    /// Update price for a held position. Returns exit reason if should exit.
    pub fn on_price_update(&self, mint: &str, price_sol: f64) -> Option<String> {
        let mut pos = self.positions.get_mut(mint)?;
        if pos.exited { return None; }

        pos.current_price_sol = price_sol;
        if price_sol > pos.peak_price_sol {
            pos.peak_price_sol = price_sol;
        }

        if let Some(reason) = check_exit(&pos) {
            pos.exited = true;
            let ret = pos.return_pct();
            let pnl = pos.unrealized_pnl_sol();

            info!(
                "[STRATEGY] EXIT:{} {} | ret={:+.1}% pnl={:+.6} SOL | {:.0}s",
                reason,
                &mint[..mint.len().min(12)],
                ret, pnl, pos.elapsed_s(),
            );

            self.total_trades.fetch_add(1, Ordering::Relaxed);
            if pnl > 0.0 {
                self.wins.fetch_add(1, Ordering::Relaxed);
            } else {
                self.losses.fetch_add(1, Ordering::Relaxed);
            }

            drop(pos);
            self.positions.remove(mint);

            return Some(reason.to_string());
        }

        None
    }

    /// Get all position infos for API.
    pub fn position_infos(&self) -> Vec<PositionInfo> {
        self.positions.iter().map(|entry| {
            let p = entry.value();
            PositionInfo {
                mint: p.mint.clone(),
                strategy: p.strategy.clone(),
                entry_price_sol: p.entry_price_sol,
                current_price_sol: p.current_price_sol,
                return_pct: p.return_pct(),
                peak_return_pct: p.peak_return_pct(),
                unrealized_pnl_sol: p.unrealized_pnl_sol(),
                elapsed_s: p.elapsed_s(),
                sol_invested: p.sol_invested,
            }
        }).collect()
    }

    /// Stats summary.
    pub fn stats_line(&self) -> String {
        let triggers = self.total_triggers.load(Ordering::Relaxed);
        let trades = self.total_trades.load(Ordering::Relaxed);
        let wins = self.wins.load(Ordering::Relaxed);
        let losses = self.losses.load(Ordering::Relaxed);
        let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
        format!(
            "triggers={} trades={} {}W/{}L ({:.0}%) pos={}",
            triggers, trades, wins, losses, wr, self.positions.len()
        )
    }
}
