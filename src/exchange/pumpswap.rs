//! PumpSwap Exchange — AMM trading for graduated tokens.
//! Adapted from RAMI/GRAD/rust_monitor/src/pumpswap_buyer.rs + pumpswap_seller.rs.
//!
//! Symbol = mint address (base58).
//! Buy amount = SOL to spend. Sell amount = token quantity (raw u64 as f64).
//!
//! PumpSwap requires pool address, token_vault, sol_vault — these are discovered
//! via RPC lookup (pool PDA = ["pool", 0u16, creator, mint, WSOL] or ["pool-v2", mint]).

use std::sync::Arc;
use std::time::Instant;

use base64::Engine;
use chrono::Utc;
use tokio::sync::Mutex;
use tracing::info;

use crate::error::AppError;
use crate::exchange::types::*;
use crate::exchange::Exchange;
use crate::solana::constants::*;
use crate::solana::rpc;
use crate::solana::tx::{self, TxInstruction};
use crate::solana::wallet::Wallet;

const MAX_BLOCKHASH_AGE_S: f64 = 30.0;
const TX_CONFIRM_TIMEOUT_MS: u64 = 20_000;
const TX_CONFIRM_POLL_MS: u64 = 500;

/// Pool data fetched from on-chain account.
struct PoolData {
    pool_address: String,
    token_vault: [u8; 32],
    sol_vault: [u8; 32],
    coin_creator: [u8; 32],
    is_mayhem: bool,
    is_cashback: bool,
    coin_token_prog: [u8; 32],
}

#[derive(Clone)]
pub struct PumpSwapExchange {
    wallet: Wallet,
    rpc_url: String,
    client: reqwest::Client,
    blockhash: Arc<Mutex<[u8; 32]>>,
    blockhash_time: Arc<Mutex<Instant>>,
}

impl PumpSwapExchange {
    pub fn new(wallet: Wallet, rpc_url: String) -> Self {
        let client = reqwest::Client::new();
        let blockhash = Arc::new(Mutex::new([0u8; 32]));
        let blockhash_time = Arc::new(Mutex::new(Instant::now()));

        let bh = blockhash.clone();
        let bt = blockhash_time.clone();
        let c = client.clone();
        let url = rpc_url.clone();
        tokio::spawn(rpc::blockhash_refresher(bh, bt, c, url));

        info!(
            "[EXCHANGE] PumpSwap initialized ({})",
            &wallet.pubkey_b58()[..12]
        );
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
                "Blockhash stale: {:.1}s",
                age
            )));
        }
        Ok(bh)
    }

    async fn wait_for_confirmation(&self, tx_hash: &str) -> (bool, bool) {
        let start = Instant::now();
        loop {
            if start.elapsed().as_millis() as u64 > TX_CONFIRM_TIMEOUT_MS {
                return (false, false);
            }
            if let Ok((true, has_err)) = rpc::get_transaction_status(&self.client, &self.rpc_url, tx_hash).await {
                return (true, has_err);
            }
            tokio::time::sleep(std::time::Duration::from_millis(TX_CONFIRM_POLL_MS)).await;
        }
    }

    /// Discover pool for a token. Uses getProgramAccounts or known PDA derivation.
    async fn discover_pool(&self, mint: &str) -> Result<PoolData, AppError> {
        let mint_bytes = tx::bs58_decode(mint).map_err(|e| AppError::Validation(e.to_string()))?;
        let pumpswap = tx::bs58_decode(PUMPSWAP_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let spl_token = tx::bs58_decode(TOKEN_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let token_2022 = tx::bs58_decode(TOKEN_2022_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;

        // Try pool-v2 PDA first
        let (pool_v2, _) = tx::find_program_address(&[b"pool-v2", &mint_bytes], &pumpswap);
        let pool_b58 = bs58::encode(&pool_v2).into_string();

        // Fetch pool + mint account data in one call
        let accounts = rpc::get_multiple_accounts(&self.client, &self.rpc_url, &[&pool_b58, mint])
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -1,
                msg: format!("Pool discovery failed: {}", e),
            })?;

        let pool_account = accounts
            .first()
            .and_then(|a| a.as_ref())
            .ok_or_else(|| AppError::ExchangeApi {
                code: -2,
                msg: format!("No pool found for token {}. Is it graduated?", mint),
            })?;

        let mint_account = accounts
            .get(1)
            .and_then(|a| a.as_ref())
            .ok_or_else(|| AppError::ExchangeApi {
                code: -2,
                msg: format!("Mint account not found: {}", mint),
            })?;

        // Parse pool data
        let pool_b64 = pool_account["data"][0].as_str().ok_or_else(|| {
            AppError::Internal("Pool account has no data".into())
        })?;
        let pool_raw = base64::engine::general_purpose::STANDARD
            .decode(pool_b64)
            .map_err(|e| AppError::Internal(format!("Base64 decode failed: {}", e)))?;

        if pool_raw.len() <= POOL_IS_CASHBACK_OFFSET {
            return Err(AppError::Internal(format!(
                "Pool data too short: {} bytes",
                pool_raw.len()
            )));
        }

        let mut coin_creator = [0u8; 32];
        coin_creator.copy_from_slice(
            &pool_raw[POOL_COIN_CREATOR_OFFSET..POOL_COIN_CREATOR_OFFSET + 32],
        );
        let is_mayhem = pool_raw[POOL_IS_MAYHEM_OFFSET] != 0;
        let is_cashback = pool_raw[POOL_IS_CASHBACK_OFFSET] != 0;

        // Detect token program from mint owner
        let mint_owner = mint_account["owner"].as_str().unwrap_or(TOKEN_PROGRAM);
        let coin_token_prog = if mint_owner == TOKEN_2022_PROGRAM {
            token_2022
        } else {
            spl_token
        };

        // Derive vault ATAs
        let ata_program = tx::bs58_decode(ATA_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let wsol = tx::bs58_decode(WSOL_MINT).map_err(|e| AppError::Internal(e.to_string()))?;

        let (token_vault, _) = tx::find_program_address(
            &[&pool_v2, &coin_token_prog, &mint_bytes],
            &ata_program,
        );
        let (sol_vault, _) = tx::find_program_address(
            &[&pool_v2, &spl_token, &wsol],
            &ata_program,
        );

        info!(
            "[PUMPSWAP] Pool discovered | {} | mayhem={} cashback={}",
            &mint[..12.min(mint.len())],
            is_mayhem,
            is_cashback
        );

        Ok(PoolData {
            pool_address: pool_b58,
            token_vault,
            sol_vault,
            coin_creator,
            is_mayhem,
            is_cashback,
            coin_token_prog,
        })
    }

    async fn execute_buy(
        &self,
        mint: &str,
        sol_lamports: u64,
        client_order_id: &str,
    ) -> Result<OrderResult, AppError> {
        let t0 = Instant::now();

        // 1. Discover pool
        let pool = self.discover_pool(mint).await?;

        // 2. Decode addresses
        let pumpswap = tx::bs58_decode(PUMPSWAP_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let global_config = tx::bs58_decode(PUMPSWAP_GLOBAL_CONFIG).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_config = tx::bs58_decode(PUMPSWAP_FEE_CONFIG).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_program = tx::bs58_decode(PUMPSWAP_FEE_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let event_authority = tx::bs58_decode(PUMPSWAP_EVENT_AUTHORITY).map_err(|e| AppError::Internal(e.to_string()))?;
        let wsol = tx::bs58_decode(WSOL_MINT).map_err(|e| AppError::Internal(e.to_string()))?;
        let spl_token = tx::bs58_decode(TOKEN_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let ata_program = tx::bs58_decode(ATA_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let system_program = tx::bs58_decode(SYSTEM_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let compute_budget = tx::bs58_decode(COMPUTE_BUDGET_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let global_vol_acc = tx::bs58_decode(PUMPSWAP_GLOBAL_VOL_ACC).map_err(|e| AppError::Internal(e.to_string()))?;
        let mint_pk = tx::bs58_decode(mint).map_err(|e| AppError::Validation(e.to_string()))?;
        let pool_pk = tx::bs58_decode(&pool.pool_address).map_err(|e| AppError::Internal(e.to_string()))?;
        let user = *self.wallet.pubkey();

        // Fee recipient selection
        let fee_list = if pool.is_mayhem {
            MAYHEM_FEE_RECIPIENTS
        } else {
            PROTOCOL_FEE_RECIPIENTS
        };
        let fee_idx = mint.as_bytes().iter().fold(0usize, |acc, &b| acc.wrapping_add(b as usize))
            % fee_list.len();
        let fee_recipient = tx::bs58_decode(fee_list[fee_idx]).map_err(|e| AppError::Internal(e.to_string()))?;
        let (fee_recipient_ata, _) =
            tx::find_program_address(&[&fee_recipient, &spl_token, &wsol], &ata_program);

        // Derive user PDAs
        let (user_base_ata, _) =
            tx::find_program_address(&[&user, &pool.coin_token_prog, &mint_pk], &ata_program);
        let (user_wsol_ata, _) =
            tx::find_program_address(&[&user, &spl_token, &wsol], &ata_program);
        let (cc_vault_auth, _) =
            tx::find_program_address(&[b"creator_vault", &pool.coin_creator], &pumpswap);
        let (cc_vault_ata, _) =
            tx::find_program_address(&[&cc_vault_auth, &spl_token, &wsol], &ata_program);
        let (user_vol_acc, _) =
            tx::find_program_address(&[b"user_volume_accumulator", &user], &pumpswap);
        let (pool_v2, _) = tx::find_program_address(&[b"pool-v2", &mint_pk], &pumpswap);

        // 3. Build instructions

        // Compute budget
        let mut cu_limit_data = vec![2u8];
        cu_limit_data.extend_from_slice(&PUMPSWAP_COMPUTE_UNIT_LIMIT.to_le_bytes());
        let mut cu_price_data = vec![3u8];
        cu_price_data.extend_from_slice(&PUMPSWAP_COMPUTE_UNIT_PRICE_MICRO_LAMPORTS.to_le_bytes());

        // Create WSOL ATA (idempotent)
        let ix_create_wsol = TxInstruction {
            program_id: ata_program,
            accounts: vec![
                (user, true, true),
                (user_wsol_ata, false, true),
                (user, false, false),
                (wsol, false, false),
                (system_program, false, false),
                (spl_token, false, false),
            ],
            data: vec![1],
        };

        // System::Transfer SOL → WSOL ATA
        let mut transfer_data = Vec::with_capacity(12);
        transfer_data.extend_from_slice(&2u32.to_le_bytes());
        transfer_data.extend_from_slice(&sol_lamports.to_le_bytes());
        let ix_transfer = TxInstruction {
            program_id: system_program,
            accounts: vec![(user, true, true), (user_wsol_ata, false, true)],
            data: transfer_data,
        };

        // Token::SyncNative
        let ix_sync = TxInstruction {
            program_id: spl_token,
            accounts: vec![(user_wsol_ata, false, true)],
            data: vec![17],
        };

        // Create token ATA (idempotent)
        let ix_create_token = TxInstruction {
            program_id: ata_program,
            accounts: vec![
                (user, true, true),
                (user_base_ata, false, true),
                (user, false, false),
                (mint_pk, false, false),
                (system_program, false, false),
                (pool.coin_token_prog, false, false),
            ],
            data: vec![1],
        };

        // Create cc_vault_ata (idempotent)
        let ix_create_cc = TxInstruction {
            program_id: ata_program,
            accounts: vec![
                (user, true, true),
                (cc_vault_ata, false, true),
                (cc_vault_auth, false, false),
                (wsol, false, false),
                (system_program, false, false),
                (spl_token, false, false),
            ],
            data: vec![1],
        };

        // PumpSwap BuyExactQuoteIn
        let mut buy_data = Vec::with_capacity(24);
        buy_data.extend_from_slice(&PUMPSWAP_BUY_DISCRIMINATOR);
        buy_data.extend_from_slice(&sol_lamports.to_le_bytes());
        buy_data.extend_from_slice(&1u64.to_le_bytes()); // min_base_amount_out = 1

        let mut buy_accounts: Vec<([u8; 32], bool, bool)> = vec![
            (pool_pk, false, true),
            (user, true, true),
            (global_config, false, false),
            (mint_pk, false, false),
            (wsol, false, false),
            (user_base_ata, false, true),
            (user_wsol_ata, false, true),
            (pool.token_vault, false, true),
            (pool.sol_vault, false, true),
            (fee_recipient, false, false),
            (fee_recipient_ata, false, true),
            (pool.coin_token_prog, false, false),
            (spl_token, false, false),
            (system_program, false, false),
            (ata_program, false, false),
            (event_authority, false, false),
            (pumpswap, false, false),
            (cc_vault_ata, false, true),
            (cc_vault_auth, false, false),
            (global_vol_acc, false, false),
            (user_vol_acc, false, true),
            (fee_config, false, false),
            (fee_program, false, false),
        ];
        if pool.is_cashback {
            let (user_vol_wsol, _) =
                tx::find_program_address(&[&user_vol_acc, &spl_token, &wsol], &ata_program);
            buy_accounts.push((user_vol_wsol, false, true));
        }
        buy_accounts.push((pool_v2, false, false));

        let ix_buy = TxInstruction {
            program_id: pumpswap,
            accounts: buy_accounts,
            data: buy_data,
        };

        // Close WSOL ATA
        let ix_close = TxInstruction {
            program_id: spl_token,
            accounts: vec![
                (user_wsol_ata, false, true),
                (user, false, true),
                (user, true, false),
            ],
            data: vec![9],
        };

        let instructions = vec![
            TxInstruction {
                program_id: compute_budget,
                accounts: vec![],
                data: cu_limit_data,
            },
            TxInstruction {
                program_id: compute_budget,
                accounts: vec![],
                data: cu_price_data,
            },
            ix_create_wsol,
            ix_transfer,
            ix_sync,
            ix_create_token,
            ix_create_cc,
            ix_buy,
            ix_close,
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
            "[PUMPSWAP] BUY sent | {} | tx={} | {:.3} SOL | {}ms",
            &mint[..12.min(mint.len())],
            &tx_hash[..16.min(tx_hash.len())],
            sol_lamports as f64 / 1e9,
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
            side: OrderSide::Buy,
            filled_qty: sol_lamports as f64 / 1e9,
            avg_price: sol_lamports as f64 / 1e9, // approximate
            status,
            timestamp: Utc::now(),
            commission: 0.0,
        })
    }

    async fn execute_sell(
        &self,
        mint: &str,
        token_amount: u64,
        client_order_id: &str,
    ) -> Result<OrderResult, AppError> {
        let t0 = Instant::now();

        let pool = self.discover_pool(mint).await?;

        let pumpswap = tx::bs58_decode(PUMPSWAP_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let global_config = tx::bs58_decode(PUMPSWAP_GLOBAL_CONFIG).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_config = tx::bs58_decode(PUMPSWAP_FEE_CONFIG).map_err(|e| AppError::Internal(e.to_string()))?;
        let fee_program = tx::bs58_decode(PUMPSWAP_FEE_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let event_authority = tx::bs58_decode(PUMPSWAP_EVENT_AUTHORITY).map_err(|e| AppError::Internal(e.to_string()))?;
        let wsol = tx::bs58_decode(WSOL_MINT).map_err(|e| AppError::Internal(e.to_string()))?;
        let spl_token = tx::bs58_decode(TOKEN_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let ata_program = tx::bs58_decode(ATA_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let system_program = tx::bs58_decode(SYSTEM_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let compute_budget = tx::bs58_decode(COMPUTE_BUDGET_PROGRAM).map_err(|e| AppError::Internal(e.to_string()))?;
        let mint_pk = tx::bs58_decode(mint).map_err(|e| AppError::Validation(e.to_string()))?;
        let pool_pk = tx::bs58_decode(&pool.pool_address).map_err(|e| AppError::Internal(e.to_string()))?;
        let user = *self.wallet.pubkey();

        let fee_list = if pool.is_mayhem { MAYHEM_FEE_RECIPIENTS } else { PROTOCOL_FEE_RECIPIENTS };
        let fee_idx = mint.as_bytes().iter().fold(0usize, |acc, &b| acc.wrapping_add(b as usize)) % fee_list.len();
        let fee_recipient = tx::bs58_decode(fee_list[fee_idx]).map_err(|e| AppError::Internal(e.to_string()))?;
        let (fee_recipient_ata, _) = tx::find_program_address(&[&fee_recipient, &spl_token, &wsol], &ata_program);

        let (user_base_ata, _) = tx::find_program_address(&[&user, &pool.coin_token_prog, &mint_pk], &ata_program);
        let (user_wsol_ata, _) = tx::find_program_address(&[&user, &spl_token, &wsol], &ata_program);
        let (cc_vault_auth, _) = tx::find_program_address(&[b"creator_vault", &pool.coin_creator], &pumpswap);
        let (cc_vault_ata, _) = tx::find_program_address(&[&cc_vault_auth, &spl_token, &wsol], &ata_program);
        let (pool_v2, _) = tx::find_program_address(&[b"pool-v2", &mint_pk], &pumpswap);

        let mut cu_limit_data = vec![2u8];
        cu_limit_data.extend_from_slice(&PUMPSWAP_COMPUTE_UNIT_LIMIT.to_le_bytes());
        let mut cu_price_data = vec![3u8];
        cu_price_data.extend_from_slice(&PUMPSWAP_COMPUTE_UNIT_PRICE_MICRO_LAMPORTS.to_le_bytes());

        let ix_create_wsol = TxInstruction {
            program_id: ata_program,
            accounts: vec![
                (user, true, true),
                (user_wsol_ata, false, true),
                (user, false, false),
                (wsol, false, false),
                (system_program, false, false),
                (spl_token, false, false),
            ],
            data: vec![1],
        };

        let ix_create_cc = TxInstruction {
            program_id: ata_program,
            accounts: vec![
                (user, true, true),
                (cc_vault_ata, false, true),
                (cc_vault_auth, false, false),
                (wsol, false, false),
                (system_program, false, false),
                (spl_token, false, false),
            ],
            data: vec![1],
        };

        let mut sell_data = Vec::with_capacity(24);
        sell_data.extend_from_slice(&PUMPSWAP_SELL_DISCRIMINATOR);
        sell_data.extend_from_slice(&token_amount.to_le_bytes());
        sell_data.extend_from_slice(&0u64.to_le_bytes()); // min_quote_amount_out = 0

        let mut sell_accounts: Vec<([u8; 32], bool, bool)> = vec![
            (pool_pk, false, true),
            (user, true, true),
            (global_config, false, false),
            (mint_pk, false, false),
            (wsol, false, false),
            (user_base_ata, false, true),
            (user_wsol_ata, false, true),
            (pool.token_vault, false, true),
            (pool.sol_vault, false, true),
            (fee_recipient, false, false),
            (fee_recipient_ata, false, true),
            (pool.coin_token_prog, false, false),
            (spl_token, false, false),
            (system_program, false, false),
            (ata_program, false, false),
            (event_authority, false, false),
            (pumpswap, false, false),
            (cc_vault_ata, false, true),
            (cc_vault_auth, false, false),
            (fee_config, false, false),
            (fee_program, false, false),
        ];
        if pool.is_cashback {
            let (user_vol_acc, _) = tx::find_program_address(&[b"user_volume_accumulator", &user], &pumpswap);
            let (user_vol_wsol, _) = tx::find_program_address(&[&user_vol_acc, &spl_token, &wsol], &ata_program);
            sell_accounts.push((user_vol_wsol, false, true));
            sell_accounts.push((user_vol_acc, false, true));
        }
        sell_accounts.push((pool_v2, false, false));

        let ix_sell = TxInstruction {
            program_id: pumpswap,
            accounts: sell_accounts,
            data: sell_data,
        };

        let ix_close = TxInstruction {
            program_id: spl_token,
            accounts: vec![
                (user_wsol_ata, false, true),
                (user, false, true),
                (user, true, false),
            ],
            data: vec![9],
        };

        let instructions = vec![
            TxInstruction { program_id: compute_budget, accounts: vec![], data: cu_limit_data },
            TxInstruction { program_id: compute_budget, accounts: vec![], data: cu_price_data },
            ix_create_wsol,
            ix_create_cc,
            ix_sell,
            ix_close,
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
            "[PUMPSWAP] SELL sent | {} | tx={} | tokens={} | {}ms",
            &mint[..12.min(mint.len())],
            &tx_hash[..16.min(tx_hash.len())],
            token_amount,
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
            avg_price: 0.0, // AMM — exact fill price determined post-execution
            status,
            timestamp: Utc::now(),
            commission: 0.0,
        })
    }
}

impl Exchange for PumpSwapExchange {
    fn name(&self) -> &str {
        "pumpswap"
    }

    async fn market_order(&self, req: &OrderRequest) -> Result<OrderResult, AppError> {
        match req.side {
            OrderSide::Buy => {
                let sol_lamports = (req.quantity * 1_000_000_000.0) as u64;
                self.execute_buy(&req.symbol, sol_lamports, &req.client_order_id)
                    .await
            }
            OrderSide::Sell => {
                let tokens = req.quantity as u64;
                self.execute_sell(&req.symbol, tokens, &req.client_order_id)
                    .await
            }
        }
    }

    async fn limit_order(&self, _req: &OrderRequest) -> Result<OrderResult, AppError> {
        Err(AppError::Validation(
            "PumpSwap AMM does not support limit orders. Use market orders.".into(),
        ))
    }

    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<(), AppError> {
        Err(AppError::Validation(
            "PumpSwap transactions are atomic. No orders to cancel.".into(),
        ))
    }

    async fn get_balance(&self) -> Result<Balance, AppError> {
        let lamports =
            rpc::get_sol_balance(&self.client, &self.rpc_url, self.wallet.pubkey_b58())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let sol = lamports as f64 / 1_000_000_000.0;
        let sol_usd = rpc::get_sol_price_usd(&self.client).await.unwrap_or(150.0);

        Ok(Balance {
            total_usd: sol * sol_usd,
            available_usd: sol * sol_usd,
            assets: vec![AssetBalance {
                asset: "SOL".into(),
                free: sol,
                locked: 0.0,
            }],
        })
    }

    async fn get_price(&self, symbol: &str) -> Result<f64, AppError> {
        // For PumpSwap, we'd need pool reserves to calculate price.
        // Use Jupiter price API as a fast approximation.
        let resp = self.client
            .get(format!("https://api.jup.ag/price/v2?ids={}", symbol))
            .send()
            .await
            .map_err(|e| AppError::ExchangeApi {
                code: -1,
                msg: format!("Price fetch failed: {}", e),
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|e| AppError::Internal(e.to_string()))?;

        let price = body["data"][symbol]["price"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| AppError::ExchangeApi {
                code: -1,
                msg: format!("No price data for {} on Jupiter", symbol),
            })?;

        Ok(price)
    }
}
