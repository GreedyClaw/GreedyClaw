use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Top-level config loaded from ~/.greedyclaw/config.toml + .env
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server: ServerConfig,
    #[serde(default)]
    pub exchange: ExchangeConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default = "default_logging")]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    #[serde(default = "default_exchange_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub testnet: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    #[serde(default = "default_max_position")]
    pub max_position_usd: f64,
    #[serde(default = "default_max_daily_loss")]
    pub max_daily_loss_usd: f64,
    #[serde(default = "default_max_positions")]
    pub max_open_positions: usize,
    #[serde(default)]
    pub allowed_symbols: Vec<String>,
    /// Max trades per minute (circuit breaker for LLM hallucination loops)
    #[serde(default = "default_rate_limit")]
    pub max_trades_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

// Defaults
fn default_server() -> ServerConfig {
    ServerConfig {
        host: default_host(),
        port: default_port(),
    }
}
fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    7878
}
fn default_exchange_name() -> String {
    "binance".into()
}
fn default_true() -> bool {
    true
}
fn default_max_position() -> f64 {
    500.0
}
fn default_max_daily_loss() -> f64 {
    100.0
}
fn default_max_positions() -> usize {
    3
}
fn default_rate_limit() -> u32 {
    10
}
fn default_log_level() -> String {
    "info".into()
}
fn default_log_format() -> String {
    "pretty".into()
}
fn default_logging() -> LoggingConfig {
    LoggingConfig {
        level: default_log_level(),
        format: default_log_format(),
    }
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            name: default_exchange_name(),
            testnet: true,
        }
    }
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_usd: default_max_position(),
            max_daily_loss_usd: default_max_daily_loss(),
            max_open_positions: default_max_positions(),
            allowed_symbols: vec![],
            max_trades_per_minute: default_rate_limit(),
        }
    }
}

/// Returns ~/.greedyclaw/ path
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".greedyclaw")
}

/// Secrets loaded from .env
#[derive(Debug, Clone)]
pub struct Secrets {
    pub binance_api_key: String,
    pub binance_secret_key: String,
    pub auth_token: String,
}

impl Secrets {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            binance_api_key: std::env::var("BINANCE_API_KEY")
                .context("BINANCE_API_KEY not set in .env")?,
            binance_secret_key: std::env::var("BINANCE_SECRET_KEY")
                .context("BINANCE_SECRET_KEY not set in .env")?,
            auth_token: std::env::var("GREEDYCLAW_AUTH_TOKEN")
                .context("GREEDYCLAW_AUTH_TOKEN not set in .env")?,
        })
    }
}

impl Config {
    /// Load config from ~/.greedyclaw/config.toml, then .env secrets.
    pub fn load() -> Result<Self> {
        let dir = config_dir();
        let config_path = dir.join("config.toml");

        // Load .env if it exists
        let env_path = dir.join(".env");
        if env_path.exists() {
            dotenvy::from_path(&env_path).ok();
        }

        // Parse config.toml or use defaults
        if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            let config: Config =
                toml::from_str(&text).with_context(|| "Failed to parse config.toml")?;
            Ok(config)
        } else {
            tracing::warn!(
                "No config.toml found at {}, using defaults. Run `greedyclaw init` to create one.",
                config_path.display()
            );
            Ok(Config {
                server: default_server(),
                exchange: ExchangeConfig::default(),
                risk: RiskConfig::default(),
                logging: default_logging(),
            })
        }
    }
}

/// Default config.toml content for `greedyclaw init`
pub const DEFAULT_CONFIG_TOML: &str = r#"[server]
host = "127.0.0.1"
port = 7878

[exchange]
name = "binance"
testnet = true

[risk]
max_position_usd = 500.0
max_daily_loss_usd = 100.0
max_open_positions = 3
allowed_symbols = ["BTCUSDT", "ETHUSDT"]
max_trades_per_minute = 10

[logging]
level = "info"
format = "pretty"
"#;

pub const DEFAULT_ENV: &str = r#"BINANCE_API_KEY=your_testnet_api_key_here
BINANCE_SECRET_KEY=your_testnet_secret_key_here
GREEDYCLAW_AUTH_TOKEN=change_me_to_random_hex_token
"#;
