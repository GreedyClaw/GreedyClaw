use crate::api::types::*;
use crate::audit::AuditEntry;
use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;

use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use super::AppState;

/// POST /trade — the core endpoint for AI agents.
pub async fn handle_trade<E: Exchange>(
    State(state): State<Arc<AppState<E>>>,
    Json(req): Json<TradeRequest>,
) -> Result<Json<TradeResponse>, AppError> {
    // 1. Parse and validate request
    let (side, order_type) = req
        .parse()
        .map_err(|e| AppError::Validation(e))?;

    let symbol = req.symbol.to_uppercase();

    // 2. Get current price for risk check
    let price = match order_type {
        OrderType::Limit => req.price.unwrap(), // validated in parse()
        OrderType::Market => state.exchange.get_price(&symbol).await?,
    };

    // 3. Pre-trade risk check
    state
        .risk
        .check_pre_trade(&symbol, side, req.amount, price)?;

    // 4. Build order request
    let client_order_id = format!("gc-{}", Uuid::new_v4().simple());
    let order_req = OrderRequest {
        symbol: symbol.clone(),
        side,
        order_type,
        quantity: req.amount,
        price: req.price,
        client_order_id: client_order_id.clone(),
    };

    // 5. Execute on exchange
    let result = match order_type {
        OrderType::Market => state.exchange.market_order(&order_req).await,
        OrderType::Limit => state.exchange.limit_order(&order_req).await,
    };

    match result {
        Ok(fill) => {
            // 6. Update risk engine with fill
            state.risk.record_fill(&fill);
            let risk_snap = state.risk.snapshot();

            // 7. Audit log
            {
                let mut audit = state.audit.lock().await;
                if let Err(e) = audit.record(&AuditEntry {
                    client_order_id: fill.client_order_id.clone(),
                    exchange_order_id: fill.exchange_order_id.clone(),
                    symbol: fill.symbol.clone(),
                    side: fill.side,
                    order_type,
                    requested_qty: req.amount,
                    filled_qty: fill.filled_qty,
                    avg_price: fill.avg_price,
                    status: fill.status,
                    commission: fill.commission,
                    risk_snapshot: risk_snap.clone(),
                    error: None,
                }) {
                    error!("[AUDIT] Failed to record trade: {}", e);
                }
            }

            // 8. Respond
            Ok(Json(TradeResponse {
                success: true,
                order_id: fill.exchange_order_id,
                client_order_id: fill.client_order_id,
                symbol: fill.symbol,
                side: fill.side,
                filled_qty: fill.filled_qty,
                avg_price: fill.avg_price,
                status: fill.status,
                commission: fill.commission,
                timestamp: fill.timestamp,
                risk: risk_snap,
            }))
        }
        Err(e) => {
            // Audit the failure too
            let risk_snap = state.risk.snapshot();
            {
                let mut audit = state.audit.lock().await;
                let _ = audit.record(&AuditEntry {
                    client_order_id: client_order_id.clone(),
                    exchange_order_id: String::new(),
                    symbol: symbol.clone(),
                    side,
                    order_type,
                    requested_qty: req.amount,
                    filled_qty: 0.0,
                    avg_price: 0.0,
                    status: OrderStatus::Rejected,
                    commission: 0.0,
                    risk_snapshot: risk_snap,
                    error: Some(e.to_string()),
                });
            }
            Err(e)
        }
    }
}
