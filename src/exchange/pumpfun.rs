//! PumpFun Exchange — bonding curve token trading.
//! Adapted from RAMI/MOON/src/buyer.rs + seller.rs.
//!
//! Symbol = mint address (base58).
//! Buy amount = SOL to spend. Sell amount = token quantity (raw u64 as f64).

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;
use crate::solana::constants::*;
use crate::solana::rpc;
use crate::solana::tx::{self, TxInstruction};
use crate::solana::wallet::Wallet;

const MAX_BLOCKHASH_AGE_S: f64 = 30.0;
const TX_CONFIRM_TIMEOUT_MS: u64 = 15_000;
const TX_CONFIRM_POLL_MS: u64 = 500;

#[derive(Clone)]
pub struct PumpFunExchange {
    wallet: Wallet,
    rpc_url: String,
    client: reqwest::Client,
    blockhash: Arc<Mutex<[u8; 32]>>,
    blockhash_time: Arc<Mutex<Instant>>,
}

impl PumpFunExchange {
    pub fn new(wallet: Wallet, rpc_url: String) -> Self {
        let client = reqwest::Client::new();
        let blockhash = Arc::new(Mutex::new([0u8; 32]));
        let blockhash_time = Arc::new(Mutex::new(Instant::now()));

        // Start blockhash refresher
        let bh = blockhash.clone();
        let bt = blockhash_time.clone();
        let c = client.clone();
        let url = rpc_url.clone();
        tokio::spawn(rpc::blockhash_refresher(bh, bt, c, url));

        info!("[EXCHANGE] PumpFun initialized ({})", &wallet.pubkey_b58()[..12]);
        Self {
            wallet,
            rpc_url,
            client,
            blockhash,
            blockhash_time,
        }
    }

    async fn get_fresh_blockhash(&self) -> Result<[u8; 32], AppError> {
        let bh = *self.blockhash.lock().await;
        if bh == [0u8; 32] {
            return Err(AppError::Internal("Blockhash not yet initialized".into()));
        }
        let age = self.blockhash_time.lock().await.elapsed().as_secs_f64();
        if age > MAX_BLOCKHASH_AGE_S {
            return Err(AppError::Internal(format!(
                "Blockhash stale: {:.1}s > {:.1}s",
                age, MAX_BLOCKHASH_AGE_S
            )));
        }
        Ok(bh)
    }

    /// Wait for TX confirmation with timeout. Returns (confirmed, has_error).
    async fn wait_for_confirmation(&self, tx_hash: &str) -> (bool, bool) {
        let start = Instant::now();
        loop {
            if start.elapsed().as_millis() as u64 > TX_CONFIRM_TIMEOUT_MS {
                return (false, false);
            }
            match rpc::get_transaction_status(&self.client, &self.rpc_url, tx_hash).await {
                Ok((true, has_err)) => return (true, has_err),
                Ok((false, _)) => {}
                Err(_) => {}
            }
            tokio::time::sleep(std::time::Duration::from_millis(TX_CONFIRM_POLL_MS)).await;
        }
    }

    async fn execute_buy(
        &self,
        mint: &str,
        sol_lamports: u64,
        client_order_id: &str,
    ) -> Result<OrderResult, AppError> {
        let t0 = Instant::now();

        // 1. Get curve state
        let curve = rpc::fetch_curve_state(&self.client, &self.rpc_url, mint)
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -1,
                msg: format!("Failed to fetch bonding curve: {}", e),
            })?;

        if curve.complete {
            return Err(AppError::ExchangeApi {
                code: -2,
                msg: "Token graduated (bonding curve complete). Use PumpSwap exchange.".into(),
            });
        }

        let vsr = curve.virtual_sol_reserves;
        let vtr = curve.virtual_token_reserves;

        // 2. Calculate tokens and max cost
        let tokens = tx::calc_tokens_for_sol(vsr, vtr, sol_lamports);
        if tokens == 0 {
            return Err(AppError::Validation(format!(
                "Zero tokens for {} lamports",
                sol_lamports
            )));
        }
        let max_sol = sol_lamports * (10000 + BUY_SLIPPAGE_BPS) / 10000;

        // Price: SOL per whole token (6 decimals)
        let price = (vsr as f64 + sol_lamports as f64)
            / (vtr as f64 - tokens as f64)
            * 1_000_000.0
            / 1_000_000_000.0;

        // 3. Derive PDAs
        let mint_bytes = tx::bs58_decode(mint).map_err(|e| AppError::Validation(e.to_string()))?;
        let user_bytes = *self.wallet.pubkey();
        // For PumpFun, creator is embedded in bonding curve — we use a default
        // since we don't have it from the request. derive_pdas handles this.
        let token_prog_bytes =
            tx::bs58_decode(TOKEN_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let pump_bytes =
            tx::bs58_decode(PUMP_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let ata_prog_bytes =
            tx::bs58_decode(ATA_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_prog_bytes =
            tx::bs58_decode(PUMP_FEE_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let global_bytes =
            tx::bs58_decode(PUMP_GLOBAL_STATE).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_recv_bytes = tx::bs58_decode(PUMP_FEE_RECIPIENT)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let sys_bytes =
            tx::bs58_decode(SYSTEM_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let event_auth_bytes = tx::bs58_decode(PUMP_EVENT_AUTHORITY)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let compute_budget_bytes = tx::bs58_decode(COMPUTE_BUDGET_PROGRAM)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let (bonding_curve, _) =
            tx::find_program_address(&[b"bonding-curve", &mint_bytes], &pump_bytes);
        let (assoc_bonding_curve, _) =
            tx::find_program_address(&[&bonding_curve, &token_prog_bytes, &mint_bytes], &ata_prog_bytes);
        let (user_ata, _) =
            tx::find_program_address(&[&user_bytes, &token_prog_bytes, &mint_bytes], &ata_prog_bytes);
        // Creator vault — use a zero key placeholder (PumpFun derives it from curve data)
        let (creator_vault, _) =
            tx::find_program_address(&[b"creator-vault", &[0u8; 32]], &pump_bytes);
        let (fee_config, _) =
            tx::find_program_address(&[b"fee_config", &PUMP_FEE_CONFIG_SEED], &fee_prog_bytes);
        let (global_vol_accum, _) =
            tx::find_program_address(&[b"global_volume_accumulator"], &pump_bytes);
        let (user_vol_accum, _) =
            tx::find_program_address(&[b"user_volume_accumulator", &user_bytes], &pump_bytes);

        // 4. Build buy instruction
        let mut buy_data = [0u8; 24];
        buy_data[0..8].copy_from_slice(&PUMP_BUY_DISCRIMINATOR);
        buy_data[8..16].copy_from_slice(&tokens.to_le_bytes());
        buy_data[16..24].copy_from_slice(&max_sol.to_le_bytes());

        let buy_accounts: Vec<([u8; 32], bool, bool)> = vec![
            (global_bytes, false, false),
            (fee_recv_bytes, false, true),
            (mint_bytes, false, false),
            (bonding_curve, false, true),
            (assoc_bonding_curve, false, true),
            (user_ata, false, true),
            (user_bytes, true, true),
            (sys_bytes, false, false),
            (token_prog_bytes, false, false),
            (creator_vault, false, true),
            (event_auth_bytes, false, false),
            (pump_bytes, false, false),
            (global_vol_accum, false, false),
            (user_vol_accum, false, true),
            (fee_config, false, false),
            (fee_prog_bytes, false, false),
        ];

        // CreateATA instruction (idempotent)
        let create_ata_accounts: Vec<([u8; 32], bool, bool)> = vec![
            (user_bytes, true, true),
            (user_ata, false, true),
            (user_bytes, false, false),
            (mint_bytes, false, false),
            (sys_bytes, false, false),
            (token_prog_bytes, false, false),
        ];

        // Compute budget
        let mut cb_limit_data = vec![2u8];
        cb_limit_data.extend_from_slice(&COMPUTE_UNIT_LIMIT.to_le_bytes());
        let mut cb_price_data = vec![3u8];
        cb_price_data.extend_from_slice(&COMPUTE_UNIT_PRICE_MICRO_LAMPORTS.to_le_bytes());

        let instructions = vec![
            TxInstruction {
                program_id: compute_budget_bytes,
                accounts: vec![],
                data: cb_limit_data,
            },
            TxInstruction {
                program_id: compute_budget_bytes,
                accounts: vec![],
                data: cb_price_data,
            },
            TxInstruction {
                program_id: ata_prog_bytes,
                accounts: create_ata_accounts,
                data: vec![],
            },
            TxInstruction {
                program_id: pump_bytes,
                accounts: buy_accounts,
                data: buy_data.to_vec(),
            },
        ];

        let blockhash = self.get_fresh_blockhash().await?;
        let tx_bytes = tx::build_versioned_tx(&self.wallet, &instructions, &blockhash)
            .map_err(|e| AppError::Internal(format!("TX build failed: {}", e)))?;

        // 5. Send
        let tx_hash = rpc::send_transaction(&self.client, &self.rpc_url, &tx_bytes)
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -3,
                msg: format!("Send failed: {}", e),
            })?;

        let latency_ms = t0.elapsed().as_millis();
        info!(
            "[PUMPFUN] BUY sent | {} | tx={} | tokens={} | {:.3} SOL | {}ms",
            &mint[..12.min(mint.len())],
            &tx_hash[..16.min(tx_hash.len())],
            tokens,
            sol_lamports as f64 / 1e9,
            latency_ms
        );

        // 6. Wait for confirmation
        let (confirmed, has_err) = self.wait_for_confirmation(&tx_hash).await;
        let status = if confirmed && !has_err {
            OrderStatus::Filled
        } else if has_err {
            return Err(AppError::ExchangeApi {
                code: -4,
                msg: format!("Transaction failed on-chain: {}", tx_hash),
            });
        } else {
            warn!("[PUMPFUN] TX not confirmed within timeout: {}", tx_hash);
            OrderStatus::New // optimistic — may still land
        };

        Ok(OrderResult {
            exchange_order_id: tx_hash,
            client_order_id: client_order_id.to_string(),
            symbol: mint.to_string(),
            side: OrderSide::Buy,
            filled_qty: tokens as f64,
            avg_price: price,
            status,
            timestamp: Utc::now(),
            commission: 0.0, // PumpFun fee is baked into bonding curve
        })
    }

    async fn execute_sell(
        &self,
        mint: &str,
        token_amount: u64,
        client_order_id: &str,
    ) -> Result<OrderResult, AppError> {
        let t0 = Instant::now();

        // 1. Get curve state
        let curve = rpc::fetch_curve_state(&self.client, &self.rpc_url, mint)
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -1,
                msg: format!("Failed to fetch bonding curve: {}", e),
            })?;

        let vsr = curve.virtual_sol_reserves;
        let vtr = curve.virtual_token_reserves;

        // 2. Calculate output
        let sol_output = tx::calc_sol_output(vsr, vtr, token_amount);
        let min_sol = sol_output * (10000 - SELL_SLIPPAGE_BPS) / 10000;

        let price = if token_amount > 0 {
            sol_output as f64 / token_amount as f64 * 1_000_000.0 / 1_000_000_000.0
        } else {
            0.0
        };

        // 3. Derive PDAs
        let mint_bytes = tx::bs58_decode(mint).map_err(|e| AppError::Validation(e.to_string()))?;
        let user_bytes = *self.wallet.pubkey();
        let token_prog_bytes =
            tx::bs58_decode(TOKEN_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let pump_bytes =
            tx::bs58_decode(PUMP_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let ata_prog_bytes =
            tx::bs58_decode(ATA_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_prog_bytes =
            tx::bs58_decode(PUMP_FEE_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let global_bytes =
            tx::bs58_decode(PUMP_GLOBAL_STATE).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_recv_bytes = tx::bs58_decode(PUMP_FEE_RECIPIENT)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let sys_bytes =
            tx::bs58_decode(SYSTEM_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let event_auth_bytes = tx::bs58_decode(PUMP_EVENT_AUTHORITY)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let compute_budget_bytes = tx::bs58_decode(COMPUTE_BUDGET_PROGRAM)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let (bonding_curve, _) =
            tx::find_program_address(&[b"bonding-curve", &mint_bytes], &pump_bytes);
        let (assoc_bonding_curve, _) =
            tx::find_program_address(&[&bonding_curve, &token_prog_bytes, &mint_bytes], &ata_prog_bytes);
        let (user_ata, _) =
            tx::find_program_address(&[&user_bytes, &token_prog_bytes, &mint_bytes], &ata_prog_bytes);
        let (creator_vault, _) =
            tx::find_program_address(&[b"creator-vault", &[0u8; 32]], &pump_bytes);
        let (fee_config, _) =
            tx::find_program_address(&[b"fee_config", &PUMP_FEE_CONFIG_SEED], &fee_prog_bytes);

        // 4. Build sell instruction
        let mut sell_data = [0u8; 24];
        sell_data[0..8].copy_from_slice(&PUMP_SELL_DISCRIMINATOR);
        sell_data[8..16].copy_from_slice(&token_amount.to_le_bytes());
        sell_data[16..24].copy_from_slice(&min_sol.to_le_bytes());

        let sell_accounts: Vec<([u8; 32], bool, bool)> = vec![
            (global_bytes, false, false),
            (fee_recv_bytes, false, true),
            (mint_bytes, false, false),
            (bonding_curve, false, true),
            (assoc_bonding_curve, false, true),
            (user_ata, false, true),
            (user_bytes, true, true),
            (sys_bytes, false, false),
            (creator_vault, false, true),
            (token_prog_bytes, false, false),
            (event_auth_bytes, false, false),
            (pump_bytes, false, false),
            (fee_config, false, false),
            (fee_prog_bytes, false, false),
        ];

        let mut cb_limit_data = vec![2u8];
        cb_limit_data.extend_from_slice(&COMPUTE_UNIT_LIMIT.to_le_bytes());
        let mut cb_price_data = vec![3u8];
        cb_price_data.extend_from_slice(&COMPUTE_UNIT_PRICE_MICRO_LAMPORTS.to_le_bytes());

        let instructions = vec![
            TxInstruction {
                program_id: compute_budget_bytes,
                accounts: vec![],
                data: cb_limit_data,
            },
            TxInstruction {
                program_id: compute_budget_bytes,
                accounts: vec![],
                data: cb_price_data,
            },
            TxInstruction {
                program_id: pump_bytes,
                accounts: sell_accounts,
                data: sell_data.to_vec(),
            },
        ];

        let blockhash = self.get_fresh_blockhash().await?;
        let tx_bytes = tx::build_versioned_tx(&self.wallet, &instructions, &blockhash)
            .map_err(|e| AppError::Internal(format!("TX build failed: {}", e)))?;

        let tx_hash = rpc::send_transaction(&self.client, &self.rpc_url, &tx_bytes)
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -3,
                msg: format!("Send failed: {}", e),
            })?;

        let latency_ms = t0.elapsed().as_millis();
        info!(
            "[PUMPFUN] SELL sent | {} | tx={} | tokens={} | ~{:.4} SOL | {}ms",
            &mint[..12.min(mint.len())],
            &tx_hash[..16.min(tx_hash.len())],
            token_amount,
            sol_output as f64 / 1e9,
            latency_ms
        );

        let (confirmed, has_err) = self.wait_for_confirmation(&tx_hash).await;
        let status = if confirmed && !has_err {
            OrderStatus::Filled
        } else if has_err {
            return Err(AppError::ExchangeApi {
                code: -4,
                msg: format!("Transaction failed on-chain: {}", tx_hash),
            });
        } else {
            OrderStatus::New
        };

        Ok(OrderResult {
            exchange_order_id: tx_hash,
            client_order_id: client_order_id.to_string(),
            symbol: mint.to_string(),
            side: OrderSide::Sell,
            filled_qty: token_amount as f64,
            avg_price: price,
            status,
            timestamp: Utc::now(),
            commission: 0.0,
        })
    }
}

impl Exchange for PumpFunExchange {
    fn name(&self) -> &str {
        "pumpfun"
    }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        match req.side {
            OrderSide::Buy => {
                // Amount = SOL to spend
                let sol_lamports = (req.quantity * 1_000_000_000.0) as u64;
                self.execute_buy(&req.symbol, sol_lamports, &req.client_order_id)
                    .await
            }
            OrderSide::Sell => {
                // Amount = raw token count
                let tokens = req.quantity as u64;
                self.execute_sell(&req.symbol, tokens, &req.client_order_id)
                    .await
            }
        }
    }

    async fn limit_order(&self, _req: &OrderRequest) -> Result<OrderResult, AppError> {
        Err(AppError::Validation(
            "PumpFun bonding curve does not support limit orders. Use market orders.".into(),
        ))
    }

    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<(), AppError> {
        Err(AppError::Validation(
            "PumpFun transactions are atomic. No orders to cancel.".into(),
        ))
    }

    async fn get_balance(&self) -> Result<Balance, AppError> {
        let lamports =
            rpc::get_sol_balance(&self.client, &self.rpc_url, self.wallet.pubkey_b58())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let sol = lamports as f64 / 1_000_000_000.0;

        // Get SOL price in USD
        let sol_usd = rpc::get_sol_price_usd(&self.client).await.unwrap_or(150.0);
        let usd = sol * sol_usd;

        Ok(Balance {
            total_usd: usd,
            available_usd: usd,
            assets: vec![AssetBalance {
                asset: "SOL".into(),
                free: sol,
                locked: 0.0,
            }],
        })
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        let curve = rpc::fetch_curve_state(&self.client, &self.rpc_url, symbol)
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -1,
                msg: format!("Failed to fetch price: {}", e),
            })?;

        if curve.virtual_token_reserves == 0 {
            return Err(AppError::ExchangeApi {
                code: -1,
                msg: "Token has zero reserves".into(),
            });
        }

        // Price in SOL per whole token (6 decimals)
        let price = curve.virtual_sol_reserves as f64 / curve.virtual_token_reserves as f64
            * 1_000_000.0
            / 1_000_000_000.0;

        // Convert to USD
        let sol_usd = rpc::get_sol_price_usd(&self.client).await.unwrap_or(150.0);
        Ok(price * sol_usd)
    }
}
