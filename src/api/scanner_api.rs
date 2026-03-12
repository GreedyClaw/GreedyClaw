//! Scanner API endpoints — start/stop/status/config/tokens.

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::exchange::Exchange;
use crate::scanner::scoring::ScannerConfig;
use crate::scanner::ScannerStatus;

use super::AppState;

/// POST /scanner/start — start the token scanner.
/// Body: { "endpoint": "https://...", "x_token": "..." }
pub async fn handle_scanner_start<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Json(req): Json<ScannerStartRequest>,
) -> Json<serde_json::Value> {
    match state.scanner.start(req.endpoint, req.x_token).await {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": "Scanner started"
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// POST /scanner/stop — stop the token scanner.
pub async fn handle_scanner_stop<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    state.scanner.stop().await;
    Json(serde_json::json!({
        "success": true,
        "message": "Scanner stopped"
    }))
}

/// GET /scanner/status — scanner status + top tokens + positions.
pub async fn handle_scanner_status<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<ScannerStatus> {
    Json(state.scanner.status())
}

/// GET /scanner/tokens — all tracked tokens with metrics.
pub async fn handle_scanner_tokens<E: Exchange>(
    State(_state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let tokens = crate::scanner::aggregator::all_snapshots();
    Json(serde_json::json!({
        "count": tokens.len(),
        "tokens": tokens
    }))
}

/// GET /scanner/config — current scanner configuration.
pub async fn handle_scanner_config_get<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<ScannerConfig> {
    let config = state.scanner.config.read().await;
    Json(config.clone())
}

/// PUT /scanner/config — update scanner configuration on the fly.
pub async fn handle_scanner_config_put<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Json(new_config): Json<ScannerConfig>,
) -> Json<serde_json::Value> {
    let mut config = state.scanner.config.write().await;
    *config = new_config;
    Json(serde_json::json!({
        "success": true,
        "message": "Scanner config updated"
    }))
}

/// GET /scanner/positions — scanner-managed positions.
pub async fn handle_scanner_positions<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let positions = state.scanner.strategy.position_infos();
    Json(serde_json::json!({
        "count": positions.len(),
        "positions": positions
    }))
}

#[derive(serde::Deserialize)]
pub struct ScannerStartRequest {
    pub endpoint: String,
    pub x_token: String,
}
