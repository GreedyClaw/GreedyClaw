use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "MARKET"),
            OrderType::Limit => write!(f, "LIMIT"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Filled,
    PartiallyFilled,
    New,
    Cancelled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    #[allow(dead_code)]
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub client_order_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderResult {
    pub exchange_order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub filled_qty: f64,
    pub avg_price: f64,
    pub status: OrderStatus,
    pub timestamp: DateTime<Utc>,
    /// Commission paid in quote asset
    pub commission: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Balance {
    pub total_usd: f64,
    pub available_usd: f64,
    pub assets: Vec<AssetBalance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}
