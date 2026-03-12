use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{info, warn};

type HmacSha256 = Hmac<Sha256>;

const TESTNET_BASE: &str = "https://testnet.binance.vision";
const PROD_BASE: &str = "https://api.binance.com";
const RECV_WINDOW: u64 = 5000;

#[derive(Clone)]
pub struct BinanceExchange {
    client: reqwest::Client,
    api_key: String,
    secret_key: String,
    base_url: String,
}

impl BinanceExchange {
    pub fn new(api_key: String, secret_key: String, testnet: bool) -> Self {
        let base_url = if testnet { TESTNET_BASE } else { PROD_BASE };
        info!(
            "[EXCHANGE] Binance initialized ({})",
            if testnet { "testnet" } else { "production" }
        );
        Self {
            client: reqwest::Client::new(),
            api_key,
            secret_key,
            base_url: base_url.to_string(),
        }
    }

    /// HMAC-SHA256 sign a query string.
    fn sign(&self, query: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.secret_key.as_bytes()).expect("HMAC accepts any key");
        mac.update(query.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Build signed query string with timestamp.
    fn signed_query(&self, params: &[(&str, &str)]) -> String {
        let ts = Utc::now().timestamp_millis().to_string();
        let mut parts: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
        parts.push(format!("recvWindow={RECV_WINDOW}"));
        parts.push(format!("timestamp={ts}"));
        let query = parts.join("&");
        let sig = self.sign(&query);
        format!("{query}&signature={sig}")
    }

    /// Send authenticated GET request.
    async fn signed_get(&self, path: &str, params: &[(&str, &str)]) -> Result<serde_json::Value, AppError> {
        let query = self.signed_query(params);
        let url = format!("{}{path}?{query}", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        self.parse_response(resp).await
    }

    /// Send authenticated POST request.
    async fn signed_post(&self, path: &str, params: &[(&str, &str)]) -> Result<serde_json::Value, AppError> {
        let query = self.signed_query(params);
        let url = format!("{}{path}", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(query)
            .send()
            .await?;

        self.parse_response(resp).await
    }

    /// Send authenticated DELETE request.
    async fn signed_delete(&self, path: &str, params: &[(&str, &str)]) -> Result<serde_json::Value, AppError> {
        let query = self.signed_query(params);
        let url = format!("{}{path}?{query}", self.base_url);

        let resp = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        self.parse_response(resp).await
    }

    /// Parse Binance API response, mapping errors to AppError.
    async fn parse_response(&self, resp: reqwest::Response) -> Result<serde_json::Value, AppError> {
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let code = body["code"].as_i64().unwrap_or(-1);
            let msg = body["msg"].as_str().unwrap_or("Unknown error").to_string();
            warn!("[EXCHANGE] Binance error {code}: {msg}");
            return Err(AppError::ExchangeApi { code, msg });
        }

        Ok(body)
    }

    /// Parse order response from Binance JSON.
    fn parse_order_result(&self, v: &serde_json::Value) -> OrderResult {
        let status_str = v["status"].as_str().unwrap_or("UNKNOWN");
        let status = match status_str {
            "FILLED" => OrderStatus::Filled,
            "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
            "NEW" => OrderStatus::New,
            "CANCELED" => OrderStatus::Cancelled,
            "REJECTED" => OrderStatus::Rejected,
            "EXPIRED" | "EXPIRED_IN_MATCH" => OrderStatus::Expired,
            _ => OrderStatus::Rejected,
        };

        // Calculate avg price from fills if available
        let (avg_price, filled_qty, commission) = if let Some(fills) = v["fills"].as_array() {
            let mut total_cost = 0.0;
            let mut total_qty = 0.0;
            let mut total_comm = 0.0;
            for fill in fills {
                let p: f64 = fill["price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let q: f64 = fill["qty"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let c: f64 = fill["commission"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                total_cost += p * q;
                total_qty += q;
                total_comm += c;
            }
            let avg = if total_qty > 0.0 {
                total_cost / total_qty
            } else {
                0.0
            };
            (avg, total_qty, total_comm)
        } else {
            let p: f64 = v["price"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let q: f64 = v["executedQty"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            (p, q, 0.0)
        };

        let ts_ms = v["transactTime"].as_i64().unwrap_or_else(|| Utc::now().timestamp_millis());

        OrderResult {
            exchange_order_id: v["orderId"].to_string(),
            client_order_id: v["clientOrderId"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            symbol: v["symbol"].as_str().unwrap_or("").to_string(),
            side: if v["side"].as_str() == Some("BUY") {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            filled_qty,
            avg_price,
            status,
            timestamp: chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_else(Utc::now),
            commission,
        }
    }
}

impl Exchange for BinanceExchange {
    fn name(&self) -> &str {
        if self.base_url.contains("testnet") {
            "binance-testnet"
        } else {
            "binance"
        }
    }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let qty_str = format!("{:.8}", req.quantity);
        let side_str = req.side.to_string();
        let params = vec![
            ("symbol", req.symbol.as_str()),
            ("side", side_str.as_str()),
            ("type", "MARKET"),
            ("quantity", &qty_str),
            ("newClientOrderId", &req.client_order_id),
            ("newOrderRespType", "FULL"),
        ];

        info!(
            "[EXCHANGE] Market {} {} {} (coid: {})",
            req.side, req.quantity, req.symbol, req.client_order_id
        );

        let resp = self.signed_post("/api/v3/order", &params).await?;
        let result = self.parse_order_result(&resp);

        info!(
            "[EXCHANGE] Filled: {} {} @ {} (oid: {})",
            result.filled_qty, result.symbol, result.avg_price, result.exchange_order_id
        );

        Ok(result)
    }

    async fn limit_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        let price = req.price.ok_or_else(|| {
            AppError::Validation("Limit order requires price field".into())
        })?;
        let qty_str = format!("{:.8}", req.quantity);
        let price_str = format!("{:.8}", price);
        let side_str = req.side.to_string();
        let params = vec![
            ("symbol", req.symbol.as_str()),
            ("side", side_str.as_str()),
            ("type", "LIMIT"),
            ("timeInForce", "GTC"),
            ("quantity", &qty_str),
            ("price", &price_str),
            ("newClientOrderId", &req.client_order_id),
            ("newOrderRespType", "FULL"),
        ];

        info!(
            "[EXCHANGE] Limit {} {} {} @ {} (coid: {})",
            req.side, req.quantity, req.symbol, price, req.client_order_id
        );

        let resp = self.signed_post("/api/v3/order", &params).await?;
        Ok(self.parse_order_result(&resp))
    }

    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), AppError> {
        let params = vec![
            ("symbol", symbol),
            ("orderId", order_id),
        ];

        info!("[EXCHANGE] Cancel order {} on {}", order_id, symbol);
        self.signed_delete("/api/v3/order", &params).await?;
        Ok(())
    }

    async fn get_balance(&self) -> Result<Balance, AppError> {
        let resp = self.signed_get("/api/v3/account", &[]).await?;

        let mut assets = Vec::new();
        let mut total_usd = 0.0;
        let mut available_usd = 0.0;

        if let Some(balances) = resp["balances"].as_array() {
            for b in balances {
                let free: f64 = b["free"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let locked: f64 = b["locked"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                if free > 0.0 || locked > 0.0 {
                    let asset_name = b["asset"].as_str().unwrap_or("").to_string();
                    // Approximate USD value for stablecoins
                    if ["USDT", "USDC", "BUSD", "FDUSD"].contains(&asset_name.as_str()) {
                        total_usd += free + locked;
                        available_usd += free;
                    }
                    assets.push(AssetBalance {
                        asset: asset_name,
                        free,
                        locked,
                    });
                }
            }
        }

        Ok(Balance {
            total_usd,
            available_usd,
            assets,
        })
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.base_url, symbol);
        let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;

        let price: f64 = resp["price"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                AppError::ExchangeApi {
                    code: -1,
                    msg: format!("Cannot parse price for {symbol}"),
                }
            })?;

        Ok(price)
    }
}
