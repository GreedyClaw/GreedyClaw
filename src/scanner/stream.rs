//! gRPC stream — connects to Yellowstone/Shyft and subscribes to PumpFun transactions.
//! Ported from RAMI/MOON/src/main.rs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::{error, info};

use crate::proto::geyser::geyser_client::GeyserClient;
use crate::proto::geyser::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterTransactions,
};

use super::aggregator;
use super::parser::{self, InstructionRef, PumpEvent, PUMPFUN_PROGRAM};
use super::scoring::{self, ScannerConfig, TriggerSignal};
use super::strategy::StrategyManager;

/// Scanner stats (atomic for lock-free reads from API).
pub struct ScannerStats {
    pub txs_received: AtomicU64,
    pub creates: AtomicU64,
    pub buys: AtomicU64,
    pub sells: AtomicU64,
    pub completes: AtomicU64,
    pub errors: AtomicU64,
    pub reconnects: AtomicU64,
}

impl ScannerStats {
    pub fn new() -> Self {
        Self {
            txs_received: AtomicU64::new(0),
            creates: AtomicU64::new(0),
            buys: AtomicU64::new(0),
            sells: AtomicU64::new(0),
            completes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            reconnects: AtomicU64::new(0),
        }
    }
}

/// Run the gRPC scanner loop. Reconnects automatically on disconnect.
pub async fn run_scanner(
    endpoint: String,
    x_token: String,
    config: Arc<tokio::sync::RwLock<ScannerConfig>>,
    strategy: Arc<StrategyManager>,
    stats: Arc<ScannerStats>,
    running: Arc<AtomicBool>,
    trigger_tx: tokio::sync::broadcast::Sender<TriggerSignal>,
) {
    // Cleanup task
    let running_cleanup = running.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        while running_cleanup.load(Ordering::Relaxed) {
            interval.tick().await;
            aggregator::cleanup_stale();
        }
    });

    // Stats printer
    let stats_print = stats.clone();
    let running_stats = running.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        while running_stats.load(Ordering::Relaxed) {
            interval.tick().await;
            info!(
                "[SCANNER] txs={} creates={} buys={} sells={} tracking={} | {}",
                stats_print.txs_received.load(Ordering::Relaxed),
                stats_print.creates.load(Ordering::Relaxed),
                stats_print.buys.load(Ordering::Relaxed),
                stats_print.sells.load(Ordering::Relaxed),
                aggregator::token_count(),
                aggregator::debug_summary(),
            );
        }
    });

    // Main reconnect loop
    while running.load(Ordering::Relaxed) {
        match connect_and_stream(
            &endpoint, &x_token,
            &config, &strategy, &stats,
            &running, &trigger_tx,
        ).await {
            Ok(()) => {
                if running.load(Ordering::Relaxed) {
                    info!("[SCANNER] Stream ended, reconnecting in 3s...");
                    stats.reconnects.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
            Err(e) => {
                error!("[SCANNER] Connection error: {}", e);
                stats.errors.fetch_add(1, Ordering::Relaxed);
                stats.reconnects.fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    info!("[SCANNER] Stopped.");
}

async fn connect_and_stream(
    endpoint: &str,
    x_token: &str,
    config: &Arc<tokio::sync::RwLock<ScannerConfig>>,
    strategy: &Arc<StrategyManager>,
    stats: &Arc<ScannerStats>,
    running: &Arc<AtomicBool>,
    trigger_tx: &tokio::sync::broadcast::Sender<TriggerSignal>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[SCANNER] Connecting to {}...", endpoint);

    let channel = Channel::from_shared(endpoint.to_string())?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect_timeout(Duration::from_secs(10))
        .tcp_nodelay(true)
        .http2_keep_alive_interval(Duration::from_secs(10))
        .keep_alive_timeout(Duration::from_secs(5))
        .connect()
        .await?;

    let token: MetadataValue<_> = x_token.parse()?;
    let mut client = GeyserClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        req.metadata_mut().insert("x-token", token.clone());
        Ok(req)
    });
    client = client.max_decoding_message_size(1024 * 1024 * 1024);

    info!("[SCANNER] Connected! Subscribing to PumpFun...");

    let request = build_subscribe_request();
    let (req_tx, req_rx) = tokio::sync::mpsc::channel(1);
    req_tx.send(request).await?;

    let response = client
        .subscribe(tokio_stream::wrappers::ReceiverStream::new(req_rx))
        .await?;

    let _keep_alive = req_tx;
    let mut stream = response.into_inner();

    info!("[SCANNER] Streaming PumpFun transactions...");

    let _last_stats = Instant::now();

    while let Some(message) = stream.next().await {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        match message {
            Ok(msg) => {
                if let Some(UpdateOneof::Transaction(tx_update)) = msg.update_oneof {
                    stats.txs_received.fetch_add(1, Ordering::Relaxed);
                    process_tx(&tx_update, stats, config, strategy, trigger_tx).await;
                }
            }
            Err(e) => {
                error!("[SCANNER] Stream error: {}", e);
                stats.errors.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    Ok(())
}

fn build_subscribe_request() -> SubscribeRequest {
    let mut transactions = HashMap::new();
    transactions.insert(
        "pumpfun".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![PUMPFUN_PROGRAM.to_string()],
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    SubscribeRequest {
        accounts: HashMap::default(),
        slots: HashMap::default(),
        transactions,
        transactions_status: HashMap::default(),
        blocks: HashMap::default(),
        blocks_meta: HashMap::default(),
        entry: HashMap::default(),
        commitment: Some(CommitmentLevel::Confirmed as i32),
        accounts_data_slice: Vec::default(),
        ping: None,
        from_slot: None,
    }
}

async fn process_tx(
    tx_update: &crate::proto::geyser::SubscribeUpdateTransaction,
    stats: &Arc<ScannerStats>,
    config: &Arc<tokio::sync::RwLock<ScannerConfig>>,
    strategy: &Arc<StrategyManager>,
    trigger_tx: &tokio::sync::broadcast::Sender<TriggerSignal>,
) {
    let tx_info = match &tx_update.transaction {
        Some(t) => t,
        None => return,
    };

    let tx = match &tx_info.transaction {
        Some(t) => t,
        None => return,
    };

    let msg = match &tx.message {
        Some(m) => m,
        None => return,
    };

    let mut all_keys: Vec<Vec<u8>> = msg.account_keys.clone();
    if let Some(meta) = &tx_info.meta {
        all_keys.extend(meta.loaded_writable_addresses.iter().cloned());
        all_keys.extend(meta.loaded_readonly_addresses.iter().cloned());
    }

    let mut instructions: Vec<InstructionRef<'_>> = msg.instructions.iter()
        .map(|ix| InstructionRef {
            program_id_index: ix.program_id_index,
            accounts: &ix.accounts,
            data: &ix.data,
        })
        .collect();

    if let Some(meta) = &tx_info.meta {
        if !meta.inner_instructions_none {
            for group in &meta.inner_instructions {
                for inner_ix in &group.instructions {
                    instructions.push(InstructionRef {
                        program_id_index: inner_ix.program_id_index,
                        accounts: &inner_ix.accounts,
                        data: &inner_ix.data,
                    });
                }
            }
        }
    }

    let events = parser::parse_transaction(&all_keys, &instructions);

    let cfg = config.read().await;

    for event in events {
        // Update stats
        match &event {
            PumpEvent::Create { .. } => { stats.creates.fetch_add(1, Ordering::Relaxed); }
            PumpEvent::Buy { .. } => { stats.buys.fetch_add(1, Ordering::Relaxed); }
            PumpEvent::Sell { .. } => { stats.sells.fetch_add(1, Ordering::Relaxed); }
            PumpEvent::Complete { .. } => { stats.completes.fetch_add(1, Ordering::Relaxed); }
        }

        // Get mint for trigger checking
        let mint = match &event {
            PumpEvent::Buy { mint, .. } => Some(mint.clone()),
            _ => None,
        };

        // Process in aggregator
        aggregator::process_event(event);

        // Check trigger on buys
        if let Some(mint) = mint {
            if let Some(stats_ref) = aggregator::TOKENS.get(&mint) {
                if let Some(signal) = scoring::check_lazarus(&mint, &stats_ref, &cfg) {
                    // Mark as triggered in aggregator
                    drop(stats_ref);
                    if let Some(mut s) = aggregator::TOKENS.get_mut(&mint) {
                        s.triggered = true;
                    }

                    // Strategy: enter position
                    strategy.on_trigger(&signal, cfg.entry_sol, cfg.max_positions);

                    // Broadcast signal
                    let _ = trigger_tx.send(signal);
                }
            }

            // Update price for held positions
            if strategy.positions.contains_key(&mint) {
                if let Some(s) = aggregator::TOKENS.get(&mint) {
                    let price_sol = s.price_sol();
                    drop(s);
                    strategy.on_price_update(&mint, price_sol);
                }
            }
        }
    }
}
