mod api;
mod audit;
mod config;
mod dashboard;
mod error;
mod exchange;
mod risk;
mod scanner;
mod server;
mod solana;

/// gRPC proto modules (Yellowstone/Geyser for PumpFun streaming).
pub mod proto {
    pub mod solana {
        pub mod storage {
            pub mod confirmed_block {
                include!(concat!(env!("OUT_DIR"), "/solana.storage.confirmed_block.rs"));
            }
        }
    }
    pub mod geyser {
        tonic::include_proto!("geyser");
    }
}

use api::AppState;
use audit::AuditLog;
use config::{Config, Secrets};
use exchange::binance::BinanceExchange;
use exchange::ccxt::CcxtExchange;
use exchange::mt5::Mt5Exchange;
use exchange::pumpfun::PumpFunExchange;
use exchange::pumpswap::PumpSwapExchange;
use exchange::{Exchange, OrderRequest, OrderSide, OrderType};
use risk::RiskEngine;
use scanner::Scanner;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Parser)]
#[command(name = "greedyclaw")]
#[command(about = "GreedyClaw — AI-native trading execution gateway")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ~/.greedyclaw/ with default config
    Init,
    /// Start the API gateway server
    Serve,
    /// Execute a trade directly from CLI (bypasses REST)
    Trade {
        /// "buy" or "sell"
        action: String,
        /// Trading pair (e.g., "BTCUSDT") or mint address for Solana
        symbol: String,
        /// Quantity (base asset for Binance, SOL for Solana buy, tokens for Solana sell)
        amount: f64,
    },
}

fn init_logging(level: &str) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("greedyclaw={level},tower_http=info")));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}

/// Generate a cryptographically secure random hex token (32 bytes = 64 hex chars).
fn generate_auth_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// Create ~/.greedyclaw/ with template files.
fn do_init() -> anyhow::Result<()> {
    let dir = config::config_dir();
    std::fs::create_dir_all(&dir)?;

    let config_path = dir.join("config.toml");
    let env_path = dir.join(".env");

    if !config_path.exists() {
        std::fs::write(&config_path, config::DEFAULT_CONFIG_TOML)?;
        println!("Created {}", config_path.display());
    } else {
        println!("Already exists: {}", config_path.display());
    }

    if !env_path.exists() {
        // Generate a cryptographically secure auth token
        let token = generate_auth_token();
        let env_content = config::DEFAULT_ENV.replace(
            "change_me_to_random_hex_token",
            &token,
        );
        std::fs::write(&env_path, env_content)?;
        println!("Created {}", env_path.display());
        println!("  Auth token generated (64-char hex, cryptographically random)");
        println!("\n  Edit {} and set your exchange API keys!", env_path.display());
    } else {
        println!("Already exists: {}", env_path.display());
    }

    // Warn if auth token is still the placeholder
    if let Ok(env_content) = std::fs::read_to_string(&env_path) {
        if env_content.contains("change_me_to_random_hex_token") {
            println!("\n  WARNING: Auth token is still the default placeholder!");
            println!("  Run `greedyclaw init` again with a fresh .env, or replace manually.");
        }
    }

    println!("\nSupported exchanges:");
    println!("  binance   — Binance Spot (testnet/production)");
    println!("  pumpfun   — PumpFun bonding curve tokens (Solana)");
    println!("  pumpswap  — PumpSwap AMM graduated tokens (Solana)");
    println!("  mt5       — MetaTrader 5 (Forex, Gold, Indices, Stocks, Crypto)");
    println!("  bybit     — Bybit (via CCXT bridge)");
    println!("  okx       — OKX (via CCXT bridge)");
    println!("  kraken    — Kraken (via CCXT bridge)");
    println!("  coinbase  — Coinbase (via CCXT bridge)");
    println!("  ...       — 100+ more via CCXT (run ccxt_bridge.py --exchange <name>)");
    println!("\nNext: edit config.toml and .env, then run `greedyclaw serve`");
    Ok(())
}

/// Resolve Solana keypair path from env, config, or default.
fn resolve_keypair_path(secrets: &Secrets, config: &Config) -> anyhow::Result<String> {
    if let Some(path) = &secrets.solana_keypair_path {
        if !path.is_empty() {
            return Ok(path.clone());
        }
    }
    if !config.solana.keypair_path.is_empty() {
        return Ok(config.solana.keypair_path.clone());
    }
    // Default: ~/.config/solana/id.json
    let default = dirs::home_dir()
        .map(|h| {
            h.join(".config")
                .join("solana")
                .join("id.json")
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| ".config/solana/id.json".into());
    Ok(default)
}

fn log_risk_config(config: &Config) {
    info!(
        "[INIT] Risk: max_position=${}, max_daily_loss=${}, max_positions={}, rate_limit={}/min",
        config.risk.max_position_usd,
        config.risk.max_daily_loss_usd,
        config.risk.max_open_positions,
        config.risk.max_trades_per_minute,
    );
}

/// Build AppState and serve — dispatches based on exchange name.
async fn build_and_serve(config: Config, secrets: &Secrets) -> anyhow::Result<()> {
    match config.exchange.name.as_str() {
        "binance" => {
            let api_key = secrets.binance_api_key.as_ref()
                .ok_or_else(|| anyhow::anyhow!("BINANCE_API_KEY not set"))?;
            let secret_key = secrets.binance_secret_key.as_ref()
                .ok_or_else(|| anyhow::anyhow!("BINANCE_SECRET_KEY not set"))?;
            let exchange = BinanceExchange::new(api_key.clone(), secret_key.clone(), config.exchange.testnet);
            let state = Arc::new(AppState {
                exchange,
                risk: RiskEngine::new(config.risk.clone()),
                audit: Mutex::new(AuditLog::new(&config::config_dir(), &secrets.auth_token)?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: {}", state.exchange.name());
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
        "pumpfun" => {
            let kp = resolve_keypair_path(secrets, &config)?;
            let wallet = solana::wallet::Wallet::from_file(&kp)?;
            let exchange = PumpFunExchange::new(wallet, config.solana.rpc_url.clone());
            let state = Arc::new(AppState {
                exchange,
                risk: RiskEngine::new(config.risk.clone()),
                audit: Mutex::new(AuditLog::new(&config::config_dir(), &secrets.auth_token)?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: {} | RPC: {}", state.exchange.name(), config.solana.rpc_url);
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
        "pumpswap" => {
            let kp = resolve_keypair_path(secrets, &config)?;
            let wallet = solana::wallet::Wallet::from_file(&kp)?;
            let exchange = PumpSwapExchange::new(wallet, config.solana.rpc_url.clone());
            let state = Arc::new(AppState {
                exchange,
                risk: RiskEngine::new(config.risk.clone()),
                audit: Mutex::new(AuditLog::new(&config::config_dir(), &secrets.auth_token)?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: {} | RPC: {}", state.exchange.name(), config.solana.rpc_url);
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
        "mt5" => {
            let bridge_url = std::env::var("MT5_BRIDGE_URL").ok();
            let exchange = Mt5Exchange::new(bridge_url);
            let state = Arc::new(AppState {
                exchange,
                risk: RiskEngine::new(config.risk.clone()),
                audit: Mutex::new(AuditLog::new(&config::config_dir(), &secrets.auth_token)?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: {} | Bridge: {}", state.exchange.name(),
                  std::env::var("MT5_BRIDGE_URL").unwrap_or_else(|_| "http://127.0.0.1:7879".into()));
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
        other => {
            // Try as CCXT exchange (bybit, okx, kraken, coinbase, etc.)
            let bridge_url = std::env::var("CCXT_BRIDGE_URL").ok();
            let exchange = CcxtExchange::new(other.to_string(), bridge_url);
            let state = Arc::new(AppState {
                exchange,
                risk: RiskEngine::new(config.risk.clone()),
                audit: Mutex::new(AuditLog::new(&config::config_dir(), &secrets.auth_token)?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: CCXT/{} | Bridge: {}", other,
                  std::env::var("CCXT_BRIDGE_URL").unwrap_or_else(|_| "http://127.0.0.1:7880".into()));
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
    }
}

/// Execute a trade from CLI.
async fn do_trade(config: Config, secrets: &Secrets, action: &str, symbol: &str, amount: f64) -> anyhow::Result<()> {
    let side = match action.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => anyhow::bail!("Invalid action '{}'. Use 'buy' or 'sell'.", action),
    };

    let coid = format!("gc-cli-{}", uuid::Uuid::new_v4().simple());
    let req = OrderRequest {
        symbol: symbol.to_string(),
        side,
        order_type: OrderType::Market,
        quantity: amount,
        price: None,
        client_order_id: coid,
    };

    match config.exchange.name.as_str() {
        "binance" => {
            let api_key = secrets.binance_api_key.as_ref()
                .ok_or_else(|| anyhow::anyhow!("BINANCE_API_KEY not set"))?;
            let secret_key = secrets.binance_secret_key.as_ref()
                .ok_or_else(|| anyhow::anyhow!("BINANCE_SECRET_KEY not set"))?;
            let ex = BinanceExchange::new(api_key.clone(), secret_key.clone(), config.exchange.testnet);
            let price = ex.get_price(&req.symbol).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Price: ${:.2}", price);
            let result = ex.market_order(&req).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            print_result(&result);
        }
        "pumpfun" => {
            let kp = resolve_keypair_path(secrets, &config)?;
            let wallet = solana::wallet::Wallet::from_file(&kp)?;
            let ex = PumpFunExchange::new(wallet, config.solana.rpc_url.clone());
            tokio::time::sleep(std::time::Duration::from_secs(1)).await; // blockhash init
            let result = ex.market_order(&req).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            print_result(&result);
        }
        "pumpswap" => {
            let kp = resolve_keypair_path(secrets, &config)?;
            let wallet = solana::wallet::Wallet::from_file(&kp)?;
            let ex = PumpSwapExchange::new(wallet, config.solana.rpc_url.clone());
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let result = ex.market_order(&req).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            print_result(&result);
        }
        "mt5" => {
            let bridge_url = std::env::var("MT5_BRIDGE_URL").ok();
            let ex = Mt5Exchange::new(bridge_url);
            let result = ex.market_order(&req).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            print_result(&result);
        }
        other => {
            // CCXT — any exchange (bybit, okx, kraken, coinbase, etc.)
            let bridge_url = std::env::var("CCXT_BRIDGE_URL").ok();
            let ex = CcxtExchange::new(other.to_string(), bridge_url);
            let result = ex.market_order(&req).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            print_result(&result);
        }
    }
    Ok(())
}

fn print_result(r: &exchange::OrderResult) {
    println!("\nOrder result:");
    println!("  Symbol:    {}", r.symbol);
    println!("  Side:      {}", r.side);
    println!("  Qty:       {:.8}", r.filled_qty);
    println!("  Price:     {:.8}", r.avg_price);
    println!("  TX/Order:  {}", r.exchange_order_id);
    println!("  Status:    {:?}", r.status);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => do_init(),
        Commands::Serve => {
            let config = Config::load()?;
            init_logging(&config.logging.level);
            let secrets = Secrets::from_env(&config.exchange.name)?;
            build_and_serve(config, &secrets).await
        }
        Commands::Trade { action, symbol, amount } => {
            let config = Config::load()?;
            init_logging(&config.logging.level);
            let secrets = Secrets::from_env(&config.exchange.name)?;
            do_trade(config, &secrets, &action, &symbol, amount).await
        }
    }
}
