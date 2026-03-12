pub mod binance;
pub mod pumpfun;
pub mod pumpswap;
pub mod types;

pub use types::*;

use crate::error::AppError;

/// Exchange abstraction. Async, transport-agnostic.
/// For Binance: REST + HMAC signing.
/// For Solana/PumpFun: TX build + sign + RPC send (future).
///
/// All methods wait for confirmation before returning —
/// the AI agent should never need to poll for status.
pub trait Exchange: Send + Sync + 'static {
    /// Exchange identifier (e.g., "binance-testnet", "pumpfun")
    fn name(&self) -> &str;

    /// Place a market order. Blocks until filled or rejected.
    fn market_order(
        &self,
        req: &OrderRequest,
    ) -> impl std::future::Future<Output = Result<OrderResult, AppError>> + Send;

    /// Place a limit order. Returns after order is accepted (not necessarily filled).
    fn limit_order(
        &self,
        req: &OrderRequest,
    ) -> impl std::future::Future<Output = Result<OrderResult, AppError>> + Send;

    /// Cancel an open order.
    fn cancel_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send;

    /// Get account balances.
    fn get_balance(
        &self,
    ) -> impl std::future::Future<Output = Result<Balance, AppError>> + Send;

    /// Get current mid/last price for a symbol.
    fn get_price(
        &self,
        symbol: &str,
    ) -> impl std::future::Future<Output = Result<f64, AppError>> + Send;
}
