//! WebSocket support for real-time dashboard updates.
//!
//! - `/ws?token=xxx` — authenticated WebSocket endpoint
//! - Broadcasts `WsEvent` to all connected clients via `tokio::sync::broadcast`
//! - Sends initial state snapshot on connect
//! - Ping/pong keepalive every 30s

use crate::api::AppState;
use crate::exchange::types::{OrderSide, OrderStatus, Position};
use crate::exchange::Exchange;
use crate::risk::RiskSnapshot;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

/// Events broadcast to all WebSocket clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// Initial snapshot sent on connect.
    Snapshot {
        risk: RiskSnapshot,
        positions: Vec<Position>,
    },
    /// After a successful trade execution.
    TradeExecuted {
        symbol: String,
        side: OrderSide,
        filled_qty: f64,
        avg_price: f64,
        status: OrderStatus,
        commission: f64,
        timestamp: DateTime<Utc>,
        risk: RiskSnapshot,
    },
    /// Position opened, closed, or updated.
    PositionUpdate {
        positions: Vec<Position>,
    },
    /// Risk threshold approaching or hit.
    RiskAlert {
        level: String,
        message: String,
    },
    /// Price tick for a symbol.
    PriceUpdate {
        symbol: String,
        price: f64,
    },
    /// Balance changed.
    BalanceUpdate {
        total_usd: f64,
        available_usd: f64,
    },
}

/// Query params for the WebSocket handshake.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// Create a new broadcast channel for WsEvents.
/// Returns (sender, _receiver). The receiver is dropped immediately;
/// new receivers are created per-client via `sender.subscribe()`.
pub fn broadcast_channel() -> broadcast::Sender<WsEvent> {
    let (tx, _rx) = broadcast::channel::<WsEvent>(256);
    tx
}

/// WebSocket upgrade handler at GET /ws?token=xxx
pub async fn ws_handler<E: Exchange + Clone>(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<Arc<AppState<E>>>,
) -> Response {
    // Validate token before upgrading
    let expected = state.auth_token.as_bytes();
    let provided = params.token.as_deref().unwrap_or("");

    let authed = !provided.is_empty()
        && !expected.is_empty()
        && provided.len() == expected.len()
        && provided.as_bytes().ct_eq(expected).unwrap_u8() == 1;

    if !authed {
        // Return 401 by not upgrading — send a close frame immediately
        return ws.on_upgrade(|mut socket| async move {
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4001,
                    reason: "unauthorized".into(),
                })))
                .await;
        });
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an authenticated WebSocket connection.
async fn handle_socket<E: Exchange + Clone>(mut socket: WebSocket, state: Arc<AppState<E>>) {
    debug!("[WS] Client connected");

    // 1. Send initial snapshot
    let risk = state.risk.snapshot();
    let positions = state.risk.get_positions();
    let snapshot = WsEvent::Snapshot { risk, positions };

    if let Ok(json) = serde_json::to_string(&snapshot) {
        if socket.send(Message::Text(json.into())).await.is_err() {
            debug!("[WS] Client disconnected during snapshot");
            return;
        }
    }

    // 2. Subscribe to broadcast channel
    let mut rx = state.ws_tx.subscribe();

    // 3. Keepalive ping interval
    let mut ping_interval = interval(Duration::from_secs(30));
    ping_interval.tick().await; // first tick is immediate, skip it

    loop {
        tokio::select! {
            // Forward broadcast events to this client
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                debug!("[WS] Client disconnected (send failed)");
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("[WS] Client lagged, skipped {} events", n);
                        // Continue — client will catch up via next poll
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("[WS] Broadcast channel closed");
                        break;
                    }
                }
            }

            // Send ping for keepalive
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    debug!("[WS] Client disconnected (ping failed)");
                    break;
                }
            }

            // Handle incoming messages from client (pong, close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        // Expected response to our ping
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond with pong
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("[WS] Client disconnected (close/none)");
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore text/binary from client
                    }
                    Some(Err(e)) => {
                        debug!("[WS] Client error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    debug!("[WS] Client handler exiting");
}
