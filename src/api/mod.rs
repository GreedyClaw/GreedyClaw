pub mod status;
pub mod trade;
pub mod types;

use crate::audit::AuditLog;
use crate::config::Config;
use crate::exchange::Exchange;
use crate::risk::RiskEngine;
use tokio::sync::Mutex;

/// Shared application state passed to all handlers via Arc.
pub struct AppState<E: Exchange> {
    pub exchange: E,
    pub risk: RiskEngine,
    pub audit: Mutex<AuditLog>,
    pub config: Config,
}
