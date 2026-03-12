mod api;
mod audit;
mod config;
mod error;
mod exchange;
mod risk;
mod server;

use api::AppState;
use audit::AuditLog;
use config::{Config, Secrets};
use exchange::binance::BinanceExchange;
use exchange::{Exchange, OrderRequest, OrderSide, OrderType};
use risk::RiskEngine;

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
        /// Trading pair, e.g., "BTCUSDT"
        symbol: String,
        /// Quantity
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

    println!("\nNext: edit .env with your Binance Testnet keys, then run `greedyclaw serve`");
    Ok(())
}

/// Build AppState with Binance exchange.
fn build_state(config: Config, secrets: &Secrets) -> anyhow::Result<Arc<AppState<BinanceExchange>>> {
    let exchange = BinanceExchange::new(
        secrets.binance_api_key.clone(),
        secrets.binance_secret_key.clone(),
        config.exchange.testnet,
    );

    let risk = RiskEngine::new(config.risk.clone());
    let audit = AuditLog::new(&config::config_dir())?;

    Ok(Arc::new(AppState {
        exchange,
        risk,
        audit: Mutex::new(audit),
        config,
    }))
}

/// Execute a trade directly from CLI.
async fn do_trade(
    state: Arc<AppState<BinanceExchange>>,
    action: &str,
    symbol: &str,
    amount: f64,
) -> anyhow::Result<()> {
    let side = match action.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => anyhow::bail!("Invalid action '{}'. Use 'buy' or 'sell'.", action),
    };

    let symbol = symbol.to_uppercase();

    // Get price for risk check
    let price = state.exchange.get_price(&symbol).await
        .map_err(|e| anyhow::anyhow!("Failed to get price: {}", e))?;

    println!("Current price for {}: ${:.2}", symbol, price);

    // Risk check
    state.risk.check_pre_trade(&symbol, side, amount, price)
        .map_err(|e| anyhow::anyhow!("Risk check failed: {}", e))?;

    // Execute
    let coid = format!("gc-cli-{}", uuid::Uuid::new_v4().simple());
    let req = OrderRequest {
        symbol: symbol.clone(),
        side,
        order_type: OrderType::Market,
        quantity: amount,
        price: None,
        client_order_id: coid,
    };

    let result = state.exchange.market_order(&req).await
        .map_err(|e| anyhow::anyhow!("Order failed: {}", e))?;

    state.risk.record_fill(&result);

    println!("\nOrder filled:");
    println!("  Symbol:    {}", result.symbol);
    println!("  Side:      {}", result.side);
    println!("  Quantity:  {:.8}", result.filled_qty);
    println!("  Price:     ${:.2}", result.avg_price);
    println!("  Order ID:  {}", result.exchange_order_id);
    println!("  Status:    {:?}", result.status);

    // Audit
    {
        let mut audit = state.audit.lock().await;
        let _ = audit.record(&audit::AuditEntry {
            client_order_id: result.client_order_id.clone(),
            exchange_order_id: result.exchange_order_id.clone(),
            symbol: result.symbol.clone(),
            side: result.side,
            order_type: OrderType::Market,
            requested_qty: amount,
            filled_qty: result.filled_qty,
            avg_price: result.avg_price,
            status: result.status,
            commission: result.commission,
            risk_snapshot: state.risk.snapshot(),
            error: None,
        });
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => do_init(),
        Commands::Serve => {
            let config = Config::load()?;
            init_logging(&config.logging.level);
            let secrets = Secrets::from_env()?;
            let state = build_state(config.clone(), &secrets)?;

            info!("[INIT] Exchange: {}", state.exchange.name());
            info!("[INIT] Risk: max_position=${}, max_daily_loss=${}, max_positions={}, rate_limit={}/min",
                config.risk.max_position_usd,
                config.risk.max_daily_loss_usd,
                config.risk.max_open_positions,
                config.risk.max_trades_per_minute,
            );

            server::serve(state, &secrets, &config.server.host, config.server.port).await
        }
        Commands::Trade {
            action,
            symbol,
            amount,
        } => {
            let config = Config::load()?;
            init_logging(&config.logging.level);
            let secrets = Secrets::from_env()?;
            let state = build_state(config, &secrets)?;
            do_trade(state, &action, &symbol, amount).await
        }
    }
}
