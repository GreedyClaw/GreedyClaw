//! CCXT exchange — connects via HTTP bridge to CCXT Python library.
//!
//! Architecture:
//!   GreedyClaw (Rust) ──HTTP──► ccxt_bridge (Python/FastAPI :7880) ──► 100+ exchanges
//!
//! Supports: Bybit, OKX, Kraken, Coinbase, Gate.io, KuCoin, Bitget, MEXC, HTX, and more.

use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{info, warn};

const DEFAULT_BRIDGE_URL: &str = "http://127.0.0.1:7880";

#[derive(Clone)]
pub struct CcxtExchange {
    client: reqwest::Client,
    bridge_url: String,
    exchange_id: String,
}

impl CcxtExchange {
    pub fn new(exchange_id: String, bridge_url: Option<String>) -> Self {
        let url = bridge_url
            .unwrap_or_else(|| DEFAULT_BRIDGE_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        info!("[EXCHANGE] CCXT/{} bridge: {}", exchange_id, url);

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("HTTP client"),
            bridge_url: url,
            exchange_id,
        }
    }
}

// ── Bridge DTOs ────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct BridgeOrderRequest {
    symbol: String,
    side: String,
    order_type: String,
    quantity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<f64>,
    client_order_id: String,
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
struct BridgeAccount {
    total_usd: f64,
    available_usd: f64,
    assets: Vec<BridgeAsset>,
}

#[derive(Deserialize)]
struct BridgeAsset {
    asset: String,
    free: f64,
    locked: f64,
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
    volume_24h: f64,
    #[allow(dead_code)]
    change_pct: f64,
    #[allow(dead_code)]
    time: String,
}

#[derive(Deserialize)]
struct BridgeError {
    detail: String,
}

// ── Helpers ────────────────────────────────────────────────────────

fn parse_status(s: &str) -> OrderStatus {
    match s {
        "Filled" => OrderStatus::Filled,
        "New" => OrderStatus::New,
        "Cancelled" => OrderStatus::Cancelled,
        "Expired" => OrderStatus::Expired,
        _ => OrderStatus::New,
    }
}

fn parse_side(s: &str) -> OrderSide {
    if s == "sell" { OrderSide::Sell } else { OrderSide::Buy }
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

async fn extract_error(resp: reqwest::Response) -> String {
    let body = resp.text().await.unwrap_or_default();
    serde_json::from_str::<BridgeError>(&body)
        .map(|e| e.detail)
        .unwrap_or(body)
}

// ── Exchange trait ─────────────────────────────────────────────────

impl Exchange for CcxtExchange {
    fn name(&self) -> &str {
        &self.exchange_id
    }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let bridge_req = BridgeOrderRequest {
            symbol: req.symbol.clone(),
            side: match req.side { OrderSide::Buy => "buy".into(), OrderSide::Sell => "sell".into() },
            order_type: "market".into(),
            quantity: req.quantity,
            price: None,
            client_order_id: req.client_order_id.clone(),
        };

        let resp = self.client
            .post(format!("{}/order", self.bridge_url))
            .json(&bridge_req)
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("CCXT bridge: {e}")))?;

        if !resp.status().is_success() {
            let detail = extract_error(resp).await;
            warn!("[CCXT/{}] Order failed: {}", self.exchange_id, detail);
            return Err(AppError::Exchange(format!("CCXT order failed: {detail}")));
        }

        let r: BridgeOrderResult = resp.json().await
            .map_err(|e| AppError::Exchange(format!("Parse: {e}")))?;

        info!("[CCXT/{}] {} {} {:.6} {} @ {:.6} → {}",
              self.exchange_id, r.side.to_uppercase(), r.symbol,
              r.filled_qty, r.status, r.avg_price, r.exchange_order_id);

        Ok(OrderResult {
            exchange_order_id: r.exchange_order_id,
            client_order_id: r.client_order_id,
            symbol: r.symbol,
            side: parse_side(&r.side),
            filled_qty: r.filled_qty,
            avg_price: r.avg_price,
            status: parse_status(&r.status),
            timestamp: parse_ts(&r.timestamp),
            commission: r.commission,
        })
    }

    async fn limit_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let price = req.price
            .ok_or_else(|| AppError::Validation("Limit order requires price".into()))?;

        let bridge_req = BridgeOrderRequest {
            symbol: req.symbol.clone(),
            side: match req.side { OrderSide::Buy => "buy".into(), OrderSide::Sell => "sell".into() },
            order_type: "limit".into(),
            quantity: req.quantity,
            price: Some(price),
            client_order_id: req.client_order_id.clone(),
        };

        let resp = self.client
            .post(format!("{}/order", self.bridge_url))
            .json(&bridge_req)
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("CCXT bridge: {e}")))?;

        if !resp.status().is_success() {
            let detail = extract_error(resp).await;
            return Err(AppError::Exchange(format!("CCXT limit failed: {detail}")));
        }

        let r: BridgeOrderResult = resp.json().await
            .map_err(|e| AppError::Exchange(format!("Parse: {e}")))?;

        Ok(OrderResult {
            exchange_order_id: r.exchange_order_id,
            client_order_id: r.client_order_id,
            symbol: r.symbol,
            side: parse_side(&r.side),
            filled_qty: r.filled_qty,
            avg_price: r.avg_price,
            status: parse_status(&r.status),
            timestamp: parse_ts(&r.timestamp),
            commission: r.commission,
        })
    }

    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), AppError> {
        let resp = self.client
            .delete(format!("{}/order/{}?symbol={}", self.bridge_url, order_id, symbol))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("CCXT bridge: {e}")))?;

        if !resp.status().is_success() {
            let detail = extract_error(resp).await;
            return Err(AppError::Exchange(format!("CCXT cancel: {detail}")));
        }

        info!("[CCXT/{}] Cancelled order {}", self.exchange_id, order_id);
        Ok(())
    }

    async fn get_balance(&self) -> Result<Balance, AppError> {
        let resp = self.client
            .get(format!("{}/account", self.bridge_url))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("CCXT bridge: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Exchange("CCXT balance failed".into()));
        }

        let acct: BridgeAccount = resp.json().await
            .map_err(|e| AppError::Exchange(format!("Parse: {e}")))?;

        Ok(Balance {
            total_usd: acct.total_usd,
            available_usd: acct.available_usd,
            assets: acct.assets.into_iter().map(|a| AssetBalance {
                asset: a.asset,
                free: a.free,
                locked: a.locked,
            }).collect(),
        })
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let resp = self.client
            .get(format!("{}/price/{}", self.bridge_url, symbol))
            .send()
            .await
            .map_err(|e| AppError::ExchangeUnreachable(format!("CCXT bridge: {e}")))?;

        if !resp.status().is_success() {
            let detail = extract_error(resp).await;
            return Err(AppError::Exchange(format!("CCXT price: {detail}")));
        }

        let p: BridgePrice = resp.json().await
            .map_err(|e| AppError::Exchange(format!("Parse: {e}")))?;

        Ok((p.bid + p.ask) / 2.0)
    }
}
