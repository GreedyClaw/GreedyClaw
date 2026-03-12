/// Scanner module — PumpFun token discovery and autonomous trading.
/// Streams PumpFun transactions via gRPC, aggregates token stats,
/// scores tokens, and optionally auto-trades via GreedyClaw's exchange layer.

pub mod aggregator;
pub mod parser;
pub mod scoring;
pub mod strategy;
pub mod stream;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use scoring::{ScannerConfig, TriggerSignal};
use strategy::StrategyManager;
use stream::ScannerStats;

/// Scanner state shared across the application.
pub struct Scanner {
    pub config: Arc<RwLock<ScannerConfig>>,
    pub strategy: Arc<StrategyManager>,
    pub stats: Arc<ScannerStats>,
    pub running: Arc<AtomicBool>,
    pub trigger_tx: broadcast::Sender<TriggerSignal>,
    task_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Scanner {
    pub fn new() -> Self {
        let (trigger_tx, _) = broadcast::channel(100);
        Self {
            config: Arc::new(RwLock::new(ScannerConfig::default())),
            strategy: Arc::new(StrategyManager::new()),
            stats: Arc::new(ScannerStats::new()),
            running: Arc::new(AtomicBool::new(false)),
            trigger_tx,
            task_handle: tokio::sync::Mutex::new(None),
        }
    }

    /// Start the scanner with the given gRPC endpoint and token.
    pub async fn start(&self, endpoint: String, x_token: String) -> Result<(), String> {
        if self.running.load(Ordering::Relaxed) {
            return Err("Scanner is already running".into());
        }

        self.running.store(true, Ordering::Relaxed);

        let config = self.config.clone();
        let strategy = self.strategy.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let trigger_tx = self.trigger_tx.clone();

        let handle = tokio::spawn(async move {
            stream::run_scanner(
                endpoint, x_token,
                config, strategy, stats,
                running, trigger_tx,
            ).await;
        });

        *self.task_handle.lock().await = Some(handle);

        info!("[SCANNER] Started");
        Ok(())
    }

    /// Stop the scanner.
    pub async fn stop(&self) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }

        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.task_handle.lock().await.take() {
            handle.abort();
        }

        info!("[SCANNER] Stopped");
    }

    /// Get scanner status for API.
    pub fn status(&self) -> ScannerStatus {
        let s = &self.stats;
        ScannerStatus {
            running: self.running.load(Ordering::Relaxed),
            tokens_tracking: aggregator::token_count(),
            txs_received: s.txs_received.load(Ordering::Relaxed),
            creates: s.creates.load(Ordering::Relaxed),
            buys: s.buys.load(Ordering::Relaxed),
            sells: s.sells.load(Ordering::Relaxed),
            completes: s.completes.load(Ordering::Relaxed),
            errors: s.errors.load(Ordering::Relaxed),
            reconnects: s.reconnects.load(Ordering::Relaxed),
            strategy: self.strategy.stats_line(),
            positions: self.strategy.position_infos(),
            top_tokens: aggregator::top_tokens(20),
        }
    }
}

/// Serializable scanner status for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct ScannerStatus {
    pub running: bool,
    pub tokens_tracking: usize,
    pub txs_received: u64,
    pub creates: u64,
    pub buys: u64,
    pub sells: u64,
    pub completes: u64,
    pub errors: u64,
    pub reconnects: u64,
    pub strategy: String,
    pub positions: Vec<strategy::PositionInfo>,
    pub top_tokens: Vec<aggregator::TokenSnapshot>,
}
