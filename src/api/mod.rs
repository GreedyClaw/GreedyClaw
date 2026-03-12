pub mod scanner_api;
pub mod status;
pub mod trade;
pub mod types;

use crate::audit::AuditLog;
use crate::config::Config;
use crate::exchange::Exchange;
use crate::risk::RiskEngine;
use crate::scanner::Scanner;
use crate::ws::WsEvent;
use tokio::sync::{broadcast, Mutex};

/// Shared application state passed to all handlers via Arc.
pub struct AppState<E: Exchange> {
    pub exchange: E,
    pub risk: RiskEngine,
    pub audit: Mutex<AuditLog>,
    pub config: Config,
    pub scanner: Scanner,
    /// Broadcast channel for WebSocket events.
    pub ws_tx: broadcast::Sender<WsEvent>,
    /// Auth token for WebSocket authentication (constant-time compared).
    pub auth_token: String,
}
