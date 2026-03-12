use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Exchange API error {code}: {msg}")]
    ExchangeApi { code: i64, msg: String },

    #[error("Risk limit exceeded: {0}")]
    RiskViolation(String),

    #[error("Rate limit: {0}")]
    RateLimit(String),

    #[error("Invalid request: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Exchange error: {0}")]
    Exchange(String),

    #[error("Exchange unreachable: {0}")]
    ExchangeUnreachable(String),

    #[error("Internal: {0}")]
    Internal(String),
}

/// LLM-friendly error response with clear reason and suggestion.
#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
    code: &'static str,
    suggestion: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, suggestion) = match &self {
            AppError::Http(_) => (
                StatusCode::BAD_GATEWAY,
                "EXCHANGE_UNREACHABLE",
                Some("Exchange API is down or unreachable. Retry in a few seconds.".into()),
            ),
            AppError::ExchangeApi { code: c, msg } => {
                let sug = match *c {
                    -1013 => Some("Order quantity is below minimum. Increase amount.".into()),
                    -1021 => Some("Timestamp sync issue. Server clock may be off.".into()),
                    -2010 => Some("Insufficient balance. Check GET /balance first.".into()),
                    -1100 => Some("Invalid parameter. Check symbol name and amount format.".into()),
                    _ => Some(format!("Binance error {}. Check Binance API docs.", c)),
                };
                let _ = msg; // used in Display
                (StatusCode::BAD_GATEWAY, "EXCHANGE_ERROR", sug)
            }
            AppError::RiskViolation(_) => (
                StatusCode::FORBIDDEN,
                "RISK_VIOLATION",
                Some("Risk limit hit. Check GET /status for current limits and usage.".into()),
            ),
            AppError::RateLimit(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT",
                Some("Too many requests. Wait before retrying.".into()),
            ),
            AppError::Validation(_) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                Some("Fix the request parameters and retry.".into()),
            ),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", None),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                Some("Include Authorization: Bearer <token> header.".into()),
            ),
            AppError::Exchange(_) => (
                StatusCode::BAD_GATEWAY,
                "EXCHANGE_ERROR",
                Some("Exchange rejected the request. Check parameters.".into()),
            ),
            AppError::ExchangeUnreachable(_) => (
                StatusCode::BAD_GATEWAY,
                "EXCHANGE_UNREACHABLE",
                Some("Exchange bridge is unreachable. Ensure mt5-bridge is running.".into()),
            ),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                Some("Server error. Check logs.".into()),
            ),
        };

        // Sanitize: never leak internal error details to clients.
        // Full error is logged server-side; client gets safe message.
        let safe_error = match &self {
            AppError::Internal(_) => "Internal server error".to_string(),
            AppError::Http(_) => "Exchange unreachable".to_string(),
            AppError::Exchange(_) => "Exchange rejected the request".to_string(),
            AppError::ExchangeUnreachable(_) => "Exchange bridge unreachable".to_string(),
            // Safe to expose (user-facing info only)
            AppError::Validation(msg) => msg.clone(),
            AppError::RiskViolation(_) => "Risk limit exceeded. Check GET /status.".to_string(),
            AppError::RateLimit(_) => "Rate limit exceeded. Wait before retrying.".to_string(),
            AppError::ExchangeApi { code: c, .. } => format!("Exchange error code {c}"),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Unauthorized => "Unauthorized".to_string(),
        };

        // Log full error server-side (never sent to client)
        tracing::warn!(error = %self, error_code = code, "API error");

        let body = ErrorResponse {
            success: false,
            error: safe_error,
            code,
            suggestion,
        };

        (status, axum::Json(body)).into_response()
    }
}
