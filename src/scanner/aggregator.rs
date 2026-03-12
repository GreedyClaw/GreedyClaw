//! In-memory token volume aggregator with peak/dip tracking and bonding curve math.
//! Ported from RAMI/MOON/src/aggregator.rs — adapted for GreedyClaw scanner.

use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use super::parser::PumpEvent;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const PUMPFUN_INITIAL_VIRTUAL_SOL: u64 = 30_000_000_000;
const PUMPFUN_INITIAL_VIRTUAL_TOKEN: u64 = 1_073_000_000_000_000;

/// Per-token statistics.
pub struct TokenStats {
    // Volume
    pub total_volume_lamports: u64,
    pub total_sell_lamports: u64,
    pub buy_count: u32,
    pub sell_count: u32,
    pub max_buy_lamports: u64,

    // Makers
    pub unique_makers: HashSet<String>,
    pub unique_sellers: HashSet<String>,

    // Metadata
    pub creator: String,
    pub start_time: Instant,
    pub creator_sold: bool,

    // Trigger state
    pub triggered: bool,

    // Bonding curve
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,

    // Price
    #[allow(dead_code)]
    pub initial_price: f64,
    pub current_price: f64,

    // Peak & Dip
    pub peak_price: f64,
    pub dip_bottom_price: f64,
    pub dip_pct: f64,

    // BC history for speed calculation
    pub bc_history: Vec<(Instant, f64)>,

    // Graduation
    pub graduated: bool,

    // Buy streak
    pub current_buy_streak: u32,
    pub max_buy_streak: u32,
    last_buy_wallet: String,
}

impl TokenStats {
    pub fn new(creator: String) -> Self {
        let init_price = PUMPFUN_INITIAL_VIRTUAL_SOL as f64 / PUMPFUN_INITIAL_VIRTUAL_TOKEN as f64;
        Self {
            total_volume_lamports: 0,
            total_sell_lamports: 0,
            buy_count: 0,
            sell_count: 0,
            max_buy_lamports: 0,
            unique_makers: HashSet::new(),
            unique_sellers: HashSet::new(),
            creator,
            start_time: Instant::now(),
            creator_sold: false,
            triggered: false,
            virtual_sol_reserves: PUMPFUN_INITIAL_VIRTUAL_SOL,
            virtual_token_reserves: PUMPFUN_INITIAL_VIRTUAL_TOKEN,
            initial_price: init_price,
            current_price: init_price,
            peak_price: init_price,
            dip_bottom_price: init_price,
            dip_pct: 0.0,
            bc_history: Vec::new(),
            graduated: false,
            current_buy_streak: 0,
            max_buy_streak: 0,
            last_buy_wallet: String::new(),
        }
    }

    pub fn volume_sol(&self) -> f64 {
        self.total_volume_lamports as f64 / LAMPORTS_PER_SOL as f64
    }

    pub fn elapsed_s(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn bc_progress_pct(&self) -> f64 {
        let real_sol = self.virtual_sol_reserves.saturating_sub(PUMPFUN_INITIAL_VIRTUAL_SOL);
        real_sol as f64 / 85_000_000_000.0 * 100.0
    }

    pub fn whale_fraction(&self) -> f64 {
        if self.total_volume_lamports > 0 {
            self.max_buy_lamports as f64 / self.total_volume_lamports as f64
        } else {
            0.0
        }
    }

    pub fn sell_ratio(&self) -> f64 {
        if self.total_volume_lamports == 0 { return 0.0; }
        self.total_sell_lamports as f64 / self.total_volume_lamports as f64
    }

    pub fn recovery_pct(&self) -> f64 {
        if self.dip_bottom_price <= 0.0 { return 0.0; }
        (self.current_price / self.dip_bottom_price - 1.0) * 100.0
    }

    pub fn price_sol(&self) -> f64 {
        self.current_price * 1_000_000.0 / LAMPORTS_PER_SOL as f64
    }

    /// Market cap in SOL (approximate)
    pub fn mcap_sol(&self) -> f64 {
        self.price_sol() * 1_000_000_000.0 // 1B total supply
    }

    /// BC speed (%/sec) over last `window_s` seconds.
    pub fn bc_speed(&self, window_s: f64) -> f64 {
        if self.bc_history.len() < 2 { return 0.0; }
        let (t_ref, bc_ref) = self.bc_history.last().unwrap();
        let cutoff = *t_ref - Duration::from_secs_f64(window_s);
        let idx = self.bc_history.partition_point(|(t, _)| *t < cutoff);
        let use_idx = if idx < self.bc_history.len() - 1 {
            idx
        } else if idx > 0 {
            idx - 1
        } else {
            return 0.0;
        };
        let (t_old, bc_old) = &self.bc_history[use_idx];
        let dt = t_ref.duration_since(*t_old).as_secs_f64();
        if dt < 0.5 { return 0.0; }
        (bc_ref - bc_old) / dt
    }

    fn update_peak_dip(&mut self) {
        if self.current_price > self.peak_price {
            self.peak_price = self.current_price;
            self.dip_bottom_price = self.current_price;
            self.dip_pct = 0.0;
        }
        if self.current_price < self.dip_bottom_price && self.peak_price > 0.0 {
            self.dip_bottom_price = self.current_price;
            self.dip_pct = (1.0 - self.dip_bottom_price / self.peak_price) * 100.0;
        }
    }

    fn push_bc_snapshot(&mut self) {
        let now = Instant::now();
        self.bc_history.push((now, self.bc_progress_pct()));
        let cutoff = now - Duration::from_secs(10);
        self.bc_history.retain(|(t, _)| *t >= cutoff);
    }
}

/// Serializable token snapshot for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct TokenSnapshot {
    pub mint: String,
    pub creator: String,
    pub age_s: f64,
    pub volume_sol: f64,
    pub bc_progress_pct: f64,
    pub bc_speed: f64,
    pub price_sol: f64,
    pub mcap_sol: f64,
    pub dip_pct: f64,
    pub recovery_pct: f64,
    pub buyers: usize,
    pub sellers: usize,
    pub sell_ratio: f64,
    pub whale_fraction: f64,
    pub buy_streak: u32,
    pub graduated: bool,
    pub triggered: bool,
    pub creator_sold: bool,
}

/// Global concurrent token map.
pub static TOKENS: LazyLock<DashMap<String, TokenStats>> = LazyLock::new(DashMap::new);

/// Process a parsed PumpFun event.
pub fn process_event(event: PumpEvent) {
    match event {
        PumpEvent::Create { mint, creator } => {
            TOKENS.entry(mint).or_insert_with(|| TokenStats::new(creator));
        }
        PumpEvent::Buy { mint, buyer, token_amount } => {
            if token_amount == 0 { return; }
            if let Some(mut stats) = TOKENS.get_mut(&mint) {
                let vtr = stats.virtual_token_reserves;
                let vsr = stats.virtual_sol_reserves;
                let new_vtr = vtr.saturating_sub(token_amount);
                if new_vtr > 0 {
                    let k = vsr as u128 * vtr as u128;
                    let new_vsr = (k / new_vtr as u128) as u64;
                    let sol_spent = new_vsr.saturating_sub(vsr);
                    stats.total_volume_lamports = stats.total_volume_lamports.saturating_add(sol_spent);
                    stats.buy_count += 1;
                    if sol_spent > stats.max_buy_lamports {
                        stats.max_buy_lamports = sol_spent;
                    }
                    stats.virtual_sol_reserves = new_vsr;
                    stats.virtual_token_reserves = new_vtr;
                    stats.current_price = new_vsr as f64 / new_vtr as f64;

                    if buyer != stats.last_buy_wallet {
                        stats.current_buy_streak += 1;
                        if stats.current_buy_streak > stats.max_buy_streak {
                            stats.max_buy_streak = stats.current_buy_streak;
                        }
                        stats.last_buy_wallet = buyer.clone();
                    }

                    stats.update_peak_dip();
                    stats.push_bc_snapshot();
                }
                stats.unique_makers.insert(buyer);
            }
        }
        PumpEvent::Sell { mint, seller, token_amount } => {
            if token_amount == 0 { return; }
            if let Some(mut stats) = TOKENS.get_mut(&mint) {
                let vtr = stats.virtual_token_reserves;
                let vsr = stats.virtual_sol_reserves;
                let new_vtr = vtr.saturating_add(token_amount);
                let k = vsr as u128 * vtr as u128;
                let new_vsr = (k / new_vtr as u128) as u64;
                let sol_received = vsr.saturating_sub(new_vsr);
                stats.total_sell_lamports = stats.total_sell_lamports.saturating_add(sol_received);
                stats.sell_count += 1;
                stats.virtual_sol_reserves = new_vsr;
                stats.virtual_token_reserves = new_vtr;
                if new_vtr > 0 {
                    stats.current_price = new_vsr as f64 / new_vtr as f64;
                }
                stats.update_peak_dip();
                stats.push_bc_snapshot();
                stats.current_buy_streak = 0;
                stats.unique_sellers.insert(seller.clone());
                if seller == stats.creator {
                    stats.creator_sold = true;
                }
            }
        }
        PumpEvent::Complete { mint } => {
            if let Some(mut stats) = TOKENS.get_mut(&mint) {
                stats.graduated = true;
            }
        }
    }
}

/// Remove stale tokens. Returns count removed.
pub fn cleanup_stale() -> usize {
    let before = TOKENS.len();
    TOKENS.retain(|_mint, stats| {
        let ttl = if stats.triggered {
            1200.0 // 20 min for triggered
        } else if stats.total_volume_lamports >= 2_000_000_000 {
            240.0 // 4 min for active
        } else {
            15.0 // 15s for dust
        };
        stats.elapsed_s() < ttl
    });
    before - TOKENS.len()
}

pub fn token_count() -> usize {
    TOKENS.len()
}

/// Get snapshots of all tracked tokens (for API).
pub fn all_snapshots() -> Vec<TokenSnapshot> {
    TOKENS.iter().map(|entry| {
        let mint = entry.key().clone();
        let s = entry.value();
        TokenSnapshot {
            mint,
            creator: s.creator.clone(),
            age_s: s.elapsed_s(),
            volume_sol: s.volume_sol(),
            bc_progress_pct: s.bc_progress_pct(),
            bc_speed: s.bc_speed(3.0),
            price_sol: s.price_sol(),
            mcap_sol: s.mcap_sol(),
            dip_pct: s.dip_pct,
            recovery_pct: s.recovery_pct(),
            buyers: s.unique_makers.len(),
            sellers: s.unique_sellers.len(),
            sell_ratio: s.sell_ratio(),
            whale_fraction: s.whale_fraction(),
            buy_streak: s.max_buy_streak,
            graduated: s.graduated,
            triggered: s.triggered,
            creator_sold: s.creator_sold,
        }
    }).collect()
}

/// Get top tokens sorted by volume (for dashboard/API).
pub fn top_tokens(limit: usize) -> Vec<TokenSnapshot> {
    let mut snaps = all_snapshots();
    snaps.sort_by(|a, b| b.volume_sol.partial_cmp(&a.volume_sol).unwrap_or(std::cmp::Ordering::Equal));
    snaps.truncate(limit);
    snaps
}

/// Debug summary line.
pub fn debug_summary() -> String {
    let mut max_dip: f64 = 0.0;
    let mut max_bc: f64 = 0.0;
    let mut count_vol3: usize = 0;

    for entry in TOKENS.iter() {
        let stats = entry.value();
        if stats.volume_sol() >= 3.0 { count_vol3 += 1; }
        if stats.dip_pct > max_dip { max_dip = stats.dip_pct; }
        if stats.bc_progress_pct() > max_bc { max_bc = stats.bc_progress_pct(); }
    }

    format!("vol>3={} max_dip={:.0}% max_bc={:.1}%", count_vol3, max_dip, max_bc)
}
