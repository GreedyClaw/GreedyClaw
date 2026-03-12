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
        std::fs::write(&env_path, config::DEFAULT_ENV)?;
        println!("Created {}", env_path.display());
        println!("\n  Edit {} and set your API keys!", env_path.display());
    } else {
        println!("Already exists: {}", env_path.display());
    }

    println!("\nSupported exchanges:");
    println!("  binance   — Binance Spot (testnet/production)");
    println!("  pumpfun   — PumpFun bonding curve tokens (Solana)");
    println!("  pumpswap  — PumpSwap AMM graduated tokens (Solana)");
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
                audit: Mutex::new(AuditLog::new(&config::config_dir())?),
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
                audit: Mutex::new(AuditLog::new(&config::config_dir())?),
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
                audit: Mutex::new(AuditLog::new(&config::config_dir())?),
                config: config.clone(),
                scanner: Scanner::new(),
            });
            info!("[INIT] Exchange: {} | RPC: {}", state.exchange.name(), config.solana.rpc_url);
            log_risk_config(&config);
            server::serve(state, secrets, &config.server.host, config.server.port).await
        }
        other => anyhow::bail!("Unknown exchange '{}'. Use: binance, pumpfun, pumpswap", other),
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
        other => anyhow::bail!("Unknown exchange '{}'", other),
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
