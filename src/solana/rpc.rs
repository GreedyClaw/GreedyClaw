/// Solana RPC helpers: blockhash, send TX, account info, balance, confirmation.
/// Adapted from RAMI/MOON/src/rpc.rs.

use anyhow::{Context, Result};
use base64::Engine;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::warn;

use super::tx::bs58_decode;

/// Fetch latest blockhash via RPC.
pub async fn fetch_blockhash(client: &reqwest::Client, rpc_url: &str) -> Result<[u8; 32]> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": "confirmed"}]
        }))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let hash_str = body["result"]["value"]["blockhash"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No blockhash in RPC response"))?;
    let bytes = bs58::decode(hash_str).into_vec()?;
    if bytes.len() != 32 {
        anyhow::bail!("Invalid blockhash length: {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Send a signed transaction via RPC. Returns TX hash.
pub async fn send_transaction(
    client: &reqwest::Client,
    rpc_url: &str,
    tx_bytes: &[u8],
) -> Result<String> {
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [tx_b64, {
                "encoding": "base64",
                "skipPreflight": true,
                "maxRetries": 3
            }]
        }))
        .send()
        .await
        .context("sendTransaction RPC failed")?;

    let body: serde_json::Value = resp.json().await.context("Failed to parse RPC response")?;

    if let Some(err) = body.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }

    Ok(body["result"].as_str().unwrap_or("unknown").to_string())
}

/// Check transaction status. Returns (confirmed, has_error).
pub async fn get_transaction_status(
    client: &reqwest::Client,
    rpc_url: &str,
    tx_hash: &str,
) -> Result<(bool, bool)> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [tx_hash, {
                "encoding": "json",
                "commitment": "confirmed",
                "maxSupportedTransactionVersion": 0
            }]
        }))
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    if body["result"].is_null() {
        return Ok((false, false));
    }
    let has_err = !body["result"]["meta"]["err"].is_null();
    Ok((true, has_err))
}

/// Bonding curve state from on-chain account data.
#[derive(Debug)]
pub struct CurveState {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

/// Fetch PumpFun bonding curve state via RPC getAccountInfo.
pub async fn fetch_curve_state(
    client: &reqwest::Client,
    rpc_url: &str,
    mint: &str,
) -> Result<CurveState> {
    let pump_bytes = bs58_decode(super::constants::PUMP_PROGRAM)?;
    let mint_bytes = bs58_decode(mint)?;
    let (bonding_curve_pda, _) =
        super::tx::find_program_address(&[b"bonding-curve", &mint_bytes], &pump_bytes);
    let bc_b58 = bs58::encode(&bonding_curve_pda).into_string();

    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [bc_b58, {"encoding": "base64"}]
        }))
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    let data_b64 = body["result"]["value"]["data"][0]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No account data for bonding curve {}", bc_b58))?;

    let data = base64::engine::general_purpose::STANDARD.decode(data_b64)?;
    if data.len() < 49 {
        anyhow::bail!("Bonding curve data too short: {} bytes", data.len());
    }

    Ok(CurveState {
        virtual_token_reserves: u64::from_le_bytes(data[8..16].try_into()?),
        virtual_sol_reserves: u64::from_le_bytes(data[16..24].try_into()?),
        real_token_reserves: u64::from_le_bytes(data[24..32].try_into()?),
        real_sol_reserves: u64::from_le_bytes(data[32..40].try_into()?),
        token_total_supply: u64::from_le_bytes(data[40..48].try_into()?),
        complete: data[48] != 0,
    })
}

/// Get SOL balance for a wallet via RPC.
pub async fn get_sol_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    pubkey_b58: &str,
) -> Result<u64> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [pubkey_b58, {"commitment": "confirmed"}]
        }))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let lamports = body["result"]["value"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("No balance in response"))?;
    Ok(lamports)
}

/// Get token balance (SPL) for a wallet via RPC.
pub async fn get_token_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    token_account_b58: &str,
) -> Result<u64> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountBalance",
            "params": [token_account_b58]
        }))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let amount_str = body["result"]["value"]["amount"]
        .as_str()
        .unwrap_or("0");
    Ok(amount_str.parse().unwrap_or(0))
}

/// Fetch multiple account infos in one RPC call.
pub async fn get_multiple_accounts(
    client: &reqwest::Client,
    rpc_url: &str,
    pubkeys: &[&str],
) -> Result<Vec<Option<serde_json::Value>>> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getMultipleAccounts",
            "params": [pubkeys, {"encoding": "base64", "commitment": "confirmed"}]
        }))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let values = body["result"]["value"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No values in getMultipleAccounts response"))?;
    Ok(values.iter().map(|v| {
        if v.is_null() { None } else { Some(v.clone()) }
    }).collect())
}

/// Background task: refresh blockhash every 400ms.
pub async fn blockhash_refresher(
    blockhash: Arc<Mutex<[u8; 32]>>,
    blockhash_time: Arc<Mutex<Instant>>,
    client: reqwest::Client,
    rpc_url: String,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(400));
    loop {
        interval.tick().await;
        match fetch_blockhash(&client, &rpc_url).await {
            Ok(hash) => {
                *blockhash.lock().await = hash;
                *blockhash_time.lock().await = Instant::now();
            }
            Err(e) => {
                warn!("[RPC] blockhash refresh failed: {}", e);
            }
        }
    }
}

/// Get current SOL price in USD from a public API (CoinGecko simple price).
pub async fn get_sol_price_usd(client: &reqwest::Client) -> Result<f64> {
    // Use Jupiter price API (no auth required, fast)
    let resp = client
        .get("https://api.jup.ag/price/v2?ids=So11111111111111111111111111111111111111112")
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let price = body["data"]["So11111111111111111111111111111111111111112"]["price"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to get SOL price from Jupiter"))?;
    Ok(price)
}
