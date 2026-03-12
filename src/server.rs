use crate::api::{self, AppState};
use crate::config::Secrets;
use crate::dashboard;
use crate::exchange::Exchange;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

/// Bearer token auth middleware.
async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from state — we store it in request extensions during setup
    let expected = req
        .extensions()
        .get::<AuthToken>()
        .map(|t| t.0.clone())
        .unwrap_or_default();

    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    if provided.is_empty() || provided != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

#[derive(Clone)]
struct AuthToken(String);

/// Build the axum router with all routes and middleware.
pub fn build_router<E: Exchange + Clone>(
    state: Arc<AppState<E>>,
    secrets: &Secrets,
) -> Router {
    let auth_token = secrets.auth_token.clone();

    let api_routes = Router::new()
        .route("/trade", post(api::trade::handle_trade::<E>))
        .route("/status", get(api::status::handle_status::<E>))
        .route("/balance", get(api::status::handle_balance::<E>))
        .route("/positions", get(api::status::handle_positions::<E>))
        .route("/price/{symbol}", get(api::status::handle_price::<E>))
        .route("/trades", get(api::status::handle_trades::<E>))
        .route("/trades/stats", get(api::status::handle_trade_stats::<E>))
        .route("/trades/pnl", get(api::status::handle_pnl_series::<E>))
        .route("/order/{id}", delete(api::status::handle_cancel::<E>))
        // Scanner endpoints
        .route("/scanner/start", post(api::scanner_api::handle_scanner_start::<E>))
        .route("/scanner/stop", post(api::scanner_api::handle_scanner_stop::<E>))
        .route("/scanner/status", get(api::scanner_api::handle_scanner_status::<E>))
        .route("/scanner/tokens", get(api::scanner_api::handle_scanner_tokens::<E>))
        .route("/scanner/config", get(api::scanner_api::handle_scanner_config_get::<E>))
        .route("/scanner/config", axum::routing::put(api::scanner_api::handle_scanner_config_put::<E>))
        .route("/scanner/positions", get(api::scanner_api::handle_scanner_positions::<E>))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(move |mut req: Request, next: Next| {
            let token = auth_token.clone();
            async move {
                req.extensions_mut().insert(AuthToken(token));
                auth_middleware(req, next).await
            }
        }));

    // Dashboard served without auth (token entered in the UI)
    Router::new()
        .route("/dashboard", get(dashboard::serve_dashboard))
        .merge(api_routes)
}

/// Start the server.
pub async fn serve<E: Exchange + Clone>(
    state: Arc<AppState<E>>,
    secrets: &Secrets,
    host: &str,
    port: u16,
) -> anyhow::Result<()> {
    let router = build_router(state, secrets);
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🦀 GreedyClaw v{} listening on {}", env!("CARGO_PKG_VERSION"), addr);
    info!("   GET  /dashboard — visual trading dashboard");
    info!("   POST /trade     — execute trades");
    info!("   GET  /status    — health + risk snapshot");
    info!("   GET  /balance   — account balances");
    info!("   GET  /positions — open positions");
    info!("   GET  /trades    — audit log");
    info!("   POST /scanner/start  — start token scanner");
    info!("   GET  /scanner/status — scanner status + tokens");

    axum::serve(listener, router).await?;
    Ok(())
}
