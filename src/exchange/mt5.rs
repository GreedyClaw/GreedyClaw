//! MetaTrader 5 exchange — connects via HTTP bridge to MT5 Python API.
//!
//! Architecture:
//!   GreedyClaw (Rust) ──HTTP──► mt5-bridge (Python/FastAPI :7879) ──► MT5 Terminal
//!
//! Supports: Forex, Gold, Indices, Stocks, Crypto — anything in MT5.

use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const DEFAULT_BRIDGE_URL: &str = "http://127.0.0.1:7879";

#[derive(Clone)]
pub struct Mt5Exchange {
    client: reqwest::Client,
    bridge_url: String,
}

impl Mt5Exchange {
    pub fn new(bridge_url: Option<String>) -> Self {
        let url = bridge_url
            .unwrap_or_else(|| DEFAULT_BRIDGE_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        info!("[EXCHANGE] MT5 bridge: {}", url);

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("HTTP client"),
            bridge_url: url,
        }
    }

    /// Check bridge is reachable.
    #[allow(dead_code)]
    async fn health_check(&self) -> Result<(), AppError> {
        let resp = self
            .client
            .get(format!("{}/health", self.bridge_url))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge unreachable: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::ExchangeUnreachable(
                "MT5 bridge health check failed".into(),
            ));
        }
        Ok(())
    }
}

// ── Bridge DTOs ────────────────────────────────────────────────────

#[derive(Serialize)]
struct BridgeOrderRequest {
    symbol: String,
    side: String,
    order_type: String,
    quantity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<f64>,
    client_order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sl: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tp: Option<f64>,
    deviation: u32,
    magic: u64,
}

#[derive(Deserialize)]
struct BridgeOrderResult {
    exchange_order_id: String,
    client_order_id: String,
    symbol: String,
    side: String,
    filled_qty: f64,
    avg_price: f64,
    status: String,
    timestamp: String,
    commission: f64,
}

#[derive(Deserialize)]
struct BridgePosition {
    #[allow(dead_code)]
    ticket: u64,
    symbol: String,
    side: String,
    quantity: f64,
    avg_entry_price: f64,
    current_price: f64,
    unrealized_pnl: f64,
    #[allow(dead_code)]
    sl: f64,
    #[allow(dead_code)]
    tp: f64,
    #[allow(dead_code)]
    magic: u64,
    #[allow(dead_code)]
    comment: String,
    #[allow(dead_code)]
    open_time: String,
}

#[derive(Deserialize)]
struct BridgeAccount {
    total_usd: f64,
    available_usd: f64,
    equity: f64,
    #[allow(dead_code)]
    margin: f64,
    #[allow(dead_code)]
    margin_free: f64,
    #[allow(dead_code)]
    margin_level: f64,
    #[allow(dead_code)]
    leverage: u32,
    currency: String,
    #[allow(dead_code)]
    server: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize)]
struct BridgePrice {
    #[allow(dead_code)]
    symbol: String,
    bid: f64,
    ask: f64,
    #[allow(dead_code)]
    last: f64,
    #[allow(dead_code)]
    spread: u32,
    #[allow(dead_code)]
    time: String,
}

#[derive(Deserialize)]
struct BridgeError {
    detail: String,
}

// ── Helper ─────────────────────────────────────────────────────────

fn parse_status(s: &str) -> OrderStatus {
    match s {
        "Filled" => OrderStatus::Filled,
        "Rejected" => OrderStatus::Rejected,
        "Cancelled" => OrderStatus::Cancelled,
        _ => OrderStatus::New,
    }
}

fn parse_side(s: &str) -> OrderSide {
    if s == "sell" {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    }
}

fn parse_timestamp(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

// ── Exchange trait ─────────────────────────────────────────────────

impl Exchange for Mt5Exchange {
    fn name(&self) -> &str {
        "mt5"
    }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let bridge_req = BridgeOrderRequest {
            symbol: req.symbol.clone(),
            side: match req.side {
                OrderSide::Buy => "buy".into(),
                OrderSide::Sell => "sell".into(),
            },
            order_type: "market".into(),
            quantity: req.quantity,
            price: None,
            client_order_id: req.client_order_id.clone(),
            sl: None,
            tp: None,
            deviation: 20,
            magic: 777777,
        };

        let resp = self
            .client
            .post(format!("{}/order", self.bridge_url))
            .json(&bridge_req)
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<BridgeError>(&body)
                .map(|e| e.detail)
                .unwrap_or(body);
            warn!("[MT5] Order failed ({}): {}", status, detail);
            return Err(AppError::Exchange(format!("MT5 order failed: {detail}")));
        }

        let result: BridgeOrderResult = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 response parse error: {e}")))?;

        info!(
            "[MT5] {} {} {:.4} {} @ {:.5} → ticket={}",
            result.side.to_uppercase(),
            result.symbol,
            result.filled_qty,
            result.status,
            result.avg_price,
            result.exchange_order_id,
        );

        Ok(OrderResult {
            exchange_order_id: result.exchange_order_id,
            client_order_id: result.client_order_id,
            symbol: result.symbol,
            side: parse_side(&result.side),
            filled_qty: result.filled_qty,
            avg_price: result.avg_price,
            status: parse_status(&result.status),
            timestamp: parse_timestamp(&result.timestamp),
            commission: result.commission,
        })
    }

    async fn limit_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let price = req
            .price
            .ok_or_else(|| AppError::Validation("Limit order requires a price".into()))?;

        let bridge_req = BridgeOrderRequest {
            symbol: req.symbol.clone(),
            side: match req.side {
                OrderSide::Buy => "buy".into(),
                OrderSide::Sell => "sell".into(),
            },
            order_type: "limit".into(),
            quantity: req.quantity,
            price: Some(price),
            client_order_id: req.client_order_id.clone(),
            sl: None,
            tp: None,
            deviation: 20,
            magic: 777777,
        };

        let resp = self
            .client
            .post(format!("{}/order", self.bridge_url))
            .json(&bridge_req)
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<BridgeError>(&body)
                .map(|e| e.detail)
                .unwrap_or(body);
            return Err(AppError::Exchange(format!("MT5 limit order failed: {detail}")));
        }

        let result: BridgeOrderResult = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 response parse: {e}")))?;

        Ok(OrderResult {
            exchange_order_id: result.exchange_order_id,
            client_order_id: result.client_order_id,
            symbol: result.symbol,
            side: parse_side(&result.side),
            filled_qty: result.filled_qty,
            avg_price: result.avg_price,
            status: parse_status(&result.status),
            timestamp: parse_timestamp(&result.timestamp),
            commission: result.commission,
        })
    }

    async fn cancel_order(&self, _symbol: &str, order_id: &str) -> Result<(), AppError> {
        let ticket: u64 = order_id
            .parse()
            .map_err(|_| AppError::Validation(format!("Invalid MT5 ticket: {order_id}")))?;

        let resp = self
            .client
            .delete(format!("{}/order/{}", self.bridge_url, ticket))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<BridgeError>(&body)
                .map(|e| e.detail)
                .unwrap_or(body);
            return Err(AppError::Exchange(format!("MT5 cancel failed: {detail}")));
        }

        info!("[MT5] Cancelled order #{}", ticket);
        Ok(())
    }

    async fn get_balance(&self) -> Result<Balance, AppError> {
        let resp = self
            .client
            .get(format!("{}/account", self.bridge_url))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Exchange("MT5 account info failed".into()));
        }

        let acct: BridgeAccount = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 parse error: {e}")))?;

        // Get open positions for asset breakdown
        let positions = self.get_mt5_positions().await.unwrap_or_default();

        let mut assets = vec![AssetBalance {
            asset: acct.currency.clone(),
            free: acct.available_usd,
            locked: acct.total_usd - acct.available_usd,
        }];

        for p in &positions {
            assets.push(AssetBalance {
                asset: format!("{} ({})", p.symbol, if p.quantity >= 0.0 { "long" } else { "short" }),
                free: p.quantity,
                locked: 0.0,
            });
        }

        Ok(Balance {
            total_usd: acct.equity,
            available_usd: acct.available_usd,
            assets,
        })
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let resp = self
            .client
            .get(format!("{}/price/{}", self.bridge_url, symbol))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<BridgeError>(&body)
                .map(|e| e.detail)
                .unwrap_or(body);
            return Err(AppError::Exchange(format!("MT5 price failed: {detail}")));
        }

        let price: BridgePrice = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 parse: {e}")))?;

        // Return mid price
        Ok((price.bid + price.ask) / 2.0)
    }
}

impl Mt5Exchange {
    /// Get positions from bridge (used internally and by status endpoints).
    pub async fn get_mt5_positions(&self) -> Result<Vec<Position>, AppError> {
        let resp = self
            .client
            .get(format!("{}/positions", self.bridge_url))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Exchange("MT5 positions failed".into()));
        }

        let positions: Vec<BridgePosition> = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 parse: {e}")))?;

        Ok(positions
            .into_iter()
            .map(|p| Position {
                symbol: p.symbol,
                quantity: if p.side == "buy" {
                    p.quantity
                } else {
                    -p.quantity
                },
                avg_entry_price: p.avg_entry_price,
                current_price: p.current_price,
                unrealized_pnl: p.unrealized_pnl,
            })
            .collect())
    }

    /// Close a position by ticket (for dashboard/API use).
    #[allow(dead_code)]
    pub async fn close_position(&self, ticket: u64) -> Result<OrderResult, AppError> {
        let resp = self
            .client
            .delete(format!("{}/position/{}", self.bridge_url, ticket))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("MT5 bridge: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<BridgeError>(&body)
                .map(|e| e.detail)
                .unwrap_or(body);
            return Err(AppError::Exchange(format!("MT5 close failed: {detail}")));
        }

        let result: BridgeOrderResult = resp
            .json()
            .await
            .map_err(|e| AppError::Exchange(format!("MT5 parse: {e}")))?;

        Ok(OrderResult {
            exchange_order_id: result.exchange_order_id,
            client_order_id: result.client_order_id,
            symbol: result.symbol,
            side: parse_side(&result.side),
            filled_qty: result.filled_qty,
            avg_price: result.avg_price,
            status: parse_status(&result.status),
            timestamp: parse_timestamp(&result.timestamp),
            commission: result.commission,
        })
    }
}
