//! Token scoring engine — LAZARUS trigger + anti-rug filters.
//! Ported from RAMI/MOON/src/trigger.rs with configurable parameters.

use serde::{Deserialize, Serialize};
use tracing::info;

use super::aggregator::TokenStats;

/// Scanner configuration (configurable via API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    // LAZARUS trigger params
    pub laz_min_age_s: f64,
    pub laz_max_age_s: f64,
    pub laz_min_bc_pct: f64,
    pub laz_max_bc_pct: f64,
    pub laz_min_vol_sol: f64,
    pub laz_min_dip_pct: f64,
    pub laz_impulse_window_s: f64,
    pub laz_min_bc_speed: f64,
    pub laz_max_sell_ratio: f64,

    // Anti-rug
    pub max_whale_fraction: f64,
    pub zombie_min_vol_sol: f64,
    pub zombie_max_bc_pct: f64,

    // Execution
    pub entry_sol: f64,
    pub max_positions: usize,
    pub auto_trade: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            // LAZARUS: Optuna v6 optimized
            laz_min_age_s: 45.0,
            laz_max_age_s: 200.0,
            laz_min_bc_pct: 5.0,
            laz_max_bc_pct: 30.0,
            laz_min_vol_sol: 7.0,
            laz_min_dip_pct: 60.0,
            laz_impulse_window_s: 3.0,
            laz_min_bc_speed: 0.9,
            laz_max_sell_ratio: 0.70,

            // Anti-rug
            max_whale_fraction: 0.40,
            zombie_min_vol_sol: 15.0,
            zombie_max_bc_pct: 10.0,

            // Execution
            entry_sol: 0.01,
            max_positions: 5,
            auto_trade: false,
        }
    }
}

/// Trigger signal emitted when a token passes all checks.
#[derive(Debug, Clone, Serialize)]
pub struct TriggerSignal {
    pub mint: String,
    pub creator: String,
    pub strategy: String,
    pub age_s: f64,
    pub volume_sol: f64,
    pub bc_pct: f64,
    pub bc_speed: f64,
    pub dip_pct: f64,
    pub recovery_pct: f64,
    pub price_sol: f64,
    pub buyers: usize,
    pub sell_ratio: f64,
    pub whale_fraction: f64,
}

/// Check anti-rug filters.
fn check_anti_rug(stats: &TokenStats, config: &ScannerConfig) -> bool {
    // Flash bundle: elapsed < 0.5s AND makers >= 8
    if stats.elapsed_s() < 0.5 && stats.unique_makers.len() >= 8 {
        return false;
    }

    // Whale concentration
    if stats.whale_fraction() > config.max_whale_fraction {
        return false;
    }

    // Sell ratio
    if stats.sell_ratio() > config.laz_max_sell_ratio {
        return false;
    }

    // Zombie: high volume but dead BC
    let zombie_vol_lamports = (config.zombie_min_vol_sol * 1_000_000_000.0) as u64;
    if stats.total_volume_lamports >= zombie_vol_lamports
        && stats.bc_progress_pct() < config.zombie_max_bc_pct
    {
        return false;
    }

    true
}

/// Check LAZARUS trigger: dip recovery with BC impulse.
pub fn check_lazarus(mint: &str, stats: &TokenStats, config: &ScannerConfig) -> Option<TriggerSignal> {
    if stats.triggered { return None; }

    let elapsed = stats.elapsed_s();
    if elapsed < config.laz_min_age_s || elapsed > config.laz_max_age_s {
        return None;
    }

    let bc = stats.bc_progress_pct();
    if bc < config.laz_min_bc_pct || bc > config.laz_max_bc_pct {
        return None;
    }

    let min_vol_lamports = (config.laz_min_vol_sol * 1_000_000_000.0) as u64;
    if stats.total_volume_lamports < min_vol_lamports {
        return None;
    }

    if stats.dip_pct < config.laz_min_dip_pct {
        return None;
    }

    if stats.bc_speed(config.laz_impulse_window_s) < config.laz_min_bc_speed {
        return None;
    }

    if !check_anti_rug(stats, config) {
        return None;
    }

    info!(
        "[SCANNER:LAZ] {} | dip={:.0}% rec={:.1}% spd={:.2}/s vol={:.1} bc={:.1}% age={:.1}s mkr={}",
        &mint[..mint.len().min(12)],
        stats.dip_pct, stats.recovery_pct(),
        stats.bc_speed(config.laz_impulse_window_s),
        stats.volume_sol(), bc, elapsed,
        stats.unique_makers.len(),
    );

    Some(TriggerSignal {
        mint: mint.to_string(),
        creator: stats.creator.clone(),
        strategy: "LAZARUS".into(),
        age_s: elapsed,
        volume_sol: stats.volume_sol(),
        bc_pct: bc,
        bc_speed: stats.bc_speed(config.laz_impulse_window_s),
        dip_pct: stats.dip_pct,
        recovery_pct: stats.recovery_pct(),
        price_sol: stats.price_sol(),
        buyers: stats.unique_makers.len(),
        sell_ratio: stats.sell_ratio(),
        whale_fraction: stats.whale_fraction(),
    })
}
