//! Trade audit log: SQLite + JSONL with fsync.
//! Each JSONL entry includes an HMAC-SHA256 integrity signature.

use crate::exchange::types::*;
use crate::risk::RiskSnapshot;

use anyhow::Result;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tracing::{error, info};

type HmacSha256 = Hmac<Sha256>;

pub struct AuditLog {
    conn: rusqlite::Connection,
    jsonl_path: PathBuf,
    /// HMAC key for audit log integrity (derived from auth token)
    hmac_key: Vec<u8>,
}

/// A single audit entry combining request, result, and risk state.
pub struct AuditEntry {
    pub client_order_id: String,
    pub exchange_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub requested_qty: f64,
    pub filled_qty: f64,
    pub avg_price: f64,
    pub status: OrderStatus,
    pub commission: f64,
    pub risk_snapshot: RiskSnapshot,
    pub error: Option<String>,
}

impl AuditLog {
    pub fn new(dir: &PathBuf, auth_token: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;

        let db_path = dir.join("trades.db");
        let conn = rusqlite::Connection::open(&db_path)?;

        // WAL mode + synchronous NORMAL for crash safety
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                client_order_id TEXT,
                exchange_order_id TEXT,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                order_type TEXT NOT NULL,
                requested_qty REAL,
                filled_qty REAL,
                avg_price REAL,
                status TEXT,
                commission REAL,
                realized_daily_pnl REAL,
                floating_pnl REAL,
                open_positions INTEGER,
                risk_snapshot TEXT,
                error TEXT,
                hmac TEXT
            )",
        )?;

        info!("[AUDIT] Initialized: {}", db_path.display());

        // Derive HMAC key from auth token (so audit integrity is tied to the gateway instance)
        let hmac_key = format!("greedyclaw-audit-{}", auth_token).into_bytes();

        Ok(Self {
            conn,
            jsonl_path: dir.join("trades.jsonl"),
            hmac_key,
        })
    }

    /// Compute HMAC-SHA256 signature for an audit entry.
    fn sign(&self, data: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.hmac_key)
            .expect("HMAC key error");
        mac.update(data.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Record a trade (success or failure).
    pub fn record(&mut self, entry: &AuditEntry) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let risk_json = serde_json::to_string(&entry.risk_snapshot).unwrap_or_default();

        // Create integrity payload: timestamp|symbol|side|qty|price|status
        let integrity_payload = format!(
            "{}|{}|{}|{}|{}|{:?}",
            now, entry.symbol, entry.side, entry.filled_qty, entry.avg_price, entry.status
        );
        let hmac_sig = self.sign(&integrity_payload);

        self.conn.execute(
            "INSERT INTO trades (
                timestamp, client_order_id, exchange_order_id,
                symbol, side, order_type,
                requested_qty, filled_qty, avg_price, status, commission,
                realized_daily_pnl, floating_pnl, open_positions,
                risk_snapshot, error, hmac
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )",
            rusqlite::params![
                now,
                entry.client_order_id,
                entry.exchange_order_id,
                entry.symbol,
                format!("{}", entry.side),
                format!("{}", entry.order_type),
                entry.requested_qty,
                entry.filled_qty,
                entry.avg_price,
                format!("{:?}", entry.status),
                entry.commission,
                entry.risk_snapshot.realized_daily_pnl,
                entry.risk_snapshot.floating_pnl,
                entry.risk_snapshot.open_positions as i64,
                risk_json,
                entry.error,
                hmac_sig,
            ],
        )?;

        // JSONL with fsync (crash-safe append) — includes HMAC
        self.write_jsonl(entry, &now, &hmac_sig);

        Ok(())
    }

    fn write_jsonl(&self, entry: &AuditEntry, timestamp: &str, hmac_sig: &str) {
        let json = serde_json::json!({
            "source": "greedyclaw",
            "timestamp": timestamp,
            "client_order_id": entry.client_order_id,
            "exchange_order_id": entry.exchange_order_id,
            "symbol": entry.symbol,
            "side": format!("{}", entry.side),
            "order_type": format!("{}", entry.order_type),
            "requested_qty": entry.requested_qty,
            "filled_qty": entry.filled_qty,
            "avg_price": entry.avg_price,
            "status": format!("{:?}", entry.status),
            "commission": entry.commission,
            "risk": entry.risk_snapshot,
            "error": entry.error,
            "hmac": hmac_sig,
        });

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.jsonl_path)
        {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", json);
                let _ = f.flush();
                let _ = f.sync_all(); // fsync for crash safety
            }
            Err(e) => error!("[AUDIT] JSONL write error: {}", e),
        }
    }

    /// Get recent trades for GET /trades endpoint.
    pub fn recent_trades(&self, limit: usize) -> Vec<serde_json::Value> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT timestamp, client_order_id, exchange_order_id,
                        symbol, side, order_type,
                        requested_qty, filled_qty, avg_price, status, commission, error
                 FROM trades ORDER BY id DESC LIMIT ?1",
            )
            .unwrap();

        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok(serde_json::json!({
                    "timestamp": row.get::<_, String>(0)?,
                    "client_order_id": row.get::<_, Option<String>>(1)?,
                    "exchange_order_id": row.get::<_, Option<String>>(2)?,
                    "symbol": row.get::<_, String>(3)?,
                    "side": row.get::<_, String>(4)?,
                    "order_type": row.get::<_, String>(5)?,
                    "requested_qty": row.get::<_, f64>(6)?,
                    "filled_qty": row.get::<_, f64>(7)?,
                    "avg_price": row.get::<_, f64>(8)?,
                    "status": row.get::<_, String>(9)?,
                    "commission": row.get::<_, f64>(10)?,
                    "error": row.get::<_, Option<String>>(11)?,
                }))
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Aggregated trade statistics for dashboard.
    pub fn trade_stats(&self) -> serde_json::Value {
        let total: i64 = self.conn
            .query_row("SELECT COUNT(*) FROM trades WHERE status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0);

        let buys: i64 = self.conn
            .query_row("SELECT COUNT(*) FROM trades WHERE side = 'buy' AND status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0);

        let sells: i64 = self.conn
            .query_row("SELECT COUNT(*) FROM trades WHERE side = 'sell' AND status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0);

        let total_volume: f64 = self.conn
            .query_row("SELECT COALESCE(SUM(filled_qty * avg_price), 0) FROM trades WHERE status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0.0);

        let total_commission: f64 = self.conn
            .query_row("SELECT COALESCE(SUM(commission), 0) FROM trades WHERE status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0.0);

        let rejected: i64 = self.conn
            .query_row("SELECT COUNT(*) FROM trades WHERE status = 'Rejected'", [], |r| r.get(0))
            .unwrap_or(0);

        // Today's trades
        let today_trades: i64 = self.conn
            .query_row(
                "SELECT COUNT(*) FROM trades WHERE status = 'Filled' AND date(timestamp) = date('now')",
                [], |r| r.get(0),
            )
            .unwrap_or(0);

        // Unique symbols traded
        let symbols: i64 = self.conn
            .query_row("SELECT COUNT(DISTINCT symbol) FROM trades WHERE status = 'Filled'", [], |r| r.get(0))
            .unwrap_or(0);

        serde_json::json!({
            "total_trades": total,
            "buys": buys,
            "sells": sells,
            "rejected": rejected,
            "total_volume_usd": total_volume,
            "total_commission": total_commission,
            "today_trades": today_trades,
            "unique_symbols": symbols,
        })
    }

    /// PnL series for equity curve chart. Returns cumulative realized PnL over time.
    pub fn pnl_series(&self) -> Vec<serde_json::Value> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, side, filled_qty, avg_price, symbol, realized_daily_pnl
             FROM trades WHERE status = 'Filled' ORDER BY id ASC"
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "timestamp": row.get::<_, String>(0)?,
                "side": row.get::<_, String>(1)?,
                "filled_qty": row.get::<_, f64>(2)?,
                "avg_price": row.get::<_, f64>(3)?,
                "symbol": row.get::<_, String>(4)?,
                "realized_pnl": row.get::<_, f64>(5)?,
            }))
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }
}
