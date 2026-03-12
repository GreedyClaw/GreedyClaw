use crate::exchange::types::*;
use crate::risk::RiskSnapshot;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trade request from AI agent.
#[derive(Debug, Deserialize)]
pub struct TradeRequest {
    /// "buy" or "sell"
    pub action: String,
    /// Trading pair, e.g., "BTCUSDT"
    pub symbol: String,
    /// Quantity in base asset
    pub amount: f64,
    /// "market" (default) or "limit"
    #[serde(default = "default_order_type")]
    pub order_type: String,
    /// Required for limit orders
    pub price: Option<f64>,
}

fn default_order_type() -> String {
    "market".into()
}

impl TradeRequest {
    /// Parse and validate into internal types.
    pub fn parse(&self) -> Result<(OrderSide, OrderType), String> {
        let side = match self.action.to_lowercase().as_str() {
            "buy" => OrderSide::Buy,
            "sell" => OrderSide::Sell,
            other => return Err(format!("Invalid action '{other}'. Use 'buy' or 'sell'.")),
        };

        let otype = match self.order_type.to_lowercase().as_str() {
            "market" => OrderType::Market,
            "limit" => OrderType::Limit,
            other => {
                return Err(format!(
                    "Invalid order_type '{other}'. Use 'market' or 'limit'."
                ))
            }
        };

        if otype == OrderType::Limit && self.price.is_none() {
            return Err("Limit orders require 'price' field.".into());
        }

        if !self.amount.is_finite() || self.amount <= 0.0 {
            return Err("'amount' must be a finite positive number.".into());
        }

        if self.symbol.is_empty() {
            return Err("'symbol' is required.".into());
        }

        // Symbol length sanity check (prevent oversized strings)
        if self.symbol.len() > 20 {
            return Err("'symbol' too long (max 20 chars).".into());
        }

        // Price validation for limit orders
        if let Some(price) = self.price {
            if !price.is_finite() || price <= 0.0 {
                return Err("'price' must be a finite positive number.".into());
            }
        }

        Ok((side, otype))
    }
}

/// Successful trade response.
#[derive(Serialize)]
pub struct TradeResponse {
    pub success: bool,
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub filled_qty: f64,
    pub avg_price: f64,
    pub status: OrderStatus,
    pub commission: f64,
    pub timestamp: DateTime<Utc>,
    pub risk: RiskSnapshot,
}

/// Status response.
#[derive(Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub exchange: String,
    pub testnet: bool,
    pub version: &'static str,
    pub risk: RiskSnapshot,
}
