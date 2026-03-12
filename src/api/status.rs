use crate::api::types::*;
use crate::error::AppError;
use crate::exchange::Exchange;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use super::AppState;

/// Query params for OHLC endpoint.
#[derive(Debug, Deserialize)]
pub struct OhlcQuery {
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_timeframe() -> String { "1h".into() }
fn default_limit() -> u32 { 100 }

/// GET /status — health check + current risk state.
pub async fn handle_status<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "ok",
        exchange: state.exchange.name().to_string(),
        testnet: state.config.exchange.testnet,
        version: env!("CARGO_PKG_VERSION"),
        risk: state.risk.snapshot(),
    })
}

/// GET /balance — account balances from exchange.
pub async fn handle_balance<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let balance = state.exchange.get_balance().await?;
    Ok(Json(serde_json::to_value(balance).unwrap()))
}

/// GET /positions — tracked positions with risk info.
pub async fn handle_positions<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let mut positions = state.risk.get_positions();

    // Enrich with current prices (best effort)
    for pos in &mut positions {
        if let Ok(price) = state.exchange.get_price(&pos.symbol).await {
            pos.current_price = price;
            pos.unrealized_pnl = (price - pos.avg_entry_price) * pos.quantity;
        }
    }

    Json(serde_json::to_value(positions).unwrap())
}

/// GET /price/:symbol — current price.
pub async fn handle_price<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let symbol = symbol.to_uppercase();
    let price = state.exchange.get_price(&symbol).await?;
    Ok(Json(serde_json::json!({
        "symbol": symbol,
        "price": price,
    })))
}

/// GET /trades — recent trades from audit log.
pub async fn handle_trades<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let audit = state.audit.lock().await;
    let trades = audit.recent_trades(50);
    Json(serde_json::json!({ "trades": trades }))
}

/// GET /trades/stats — aggregated trade statistics for dashboard.
pub async fn handle_trade_stats<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let audit = state.audit.lock().await;
    Json(audit.trade_stats())
}

/// GET /trades/pnl — PnL series for equity curve chart.
pub async fn handle_pnl_series<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
) -> Json<serde_json::Value> {
    let audit = state.audit.lock().await;
    let series = audit.pnl_series();
    Json(serde_json::json!({ "series": series }))
}

/// DELETE /order/:id — cancel open order.
pub async fn handle_cancel<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Need symbol to cancel — try to find from context or require query param
    // For MVP, expect format "SYMBOL:ORDER_ID"
    let parts: Vec<&str> = order_id.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AppError::Validation(
            "Cancel requires format 'SYMBOL:ORDER_ID' (e.g., 'BTCUSDT:12345')".into(),
        ));
    }

    state.exchange.cancel_order(parts[0], parts[1]).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "cancelled": order_id,
    })))
}

/// GET /ohlc/:symbol — OHLCV candle data.
pub async fn handle_ohlc<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Path(symbol): Path<String>,
    Query(params): Query<OhlcQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let symbol = symbol.to_uppercase();
    let limit = params.limit.min(1000).max(1);
    let candles = state.exchange.get_ohlc(&symbol, &params.timeframe, limit).await?;
    Ok(Json(serde_json::json!({
        "symbol": symbol,
        "timeframe": params.timeframe,
        "count": candles.len(),
        "candles": candles,
    })))
}
