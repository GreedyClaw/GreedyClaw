//! Solana transaction building: PDA derivation, MessageV0, compact-u16.
//! Merged from RAMI/MOON/src/rpc.rs + RAMI/GRAD/rust_monitor/src/tx.rs.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::wallet::Wallet;

// ── PDA Derivation ──

/// Solana find_program_address: iterate nonce 255→0, find first off-curve hash.
pub fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> ([u8; 32], u8) {
    for nonce in (0..=255u8).rev() {
        if let Some(pda) = try_find_pda(seeds, nonce, program_id) {
            return (pda, nonce);
        }
    }
    tracing::warn!("[TX] Failed to find PDA, returning zero");
    ([0u8; 32], 0)
}

fn try_find_pda(seeds: &[&[u8]], nonce: u8, program_id: &[u8; 32]) -> Option<[u8; 32]> {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update([nonce]);
    hasher.update(program_id);
    hasher.update(b"ProgramDerivedAddress");
    let hash: [u8; 32] = hasher.finalize().into();

    if !is_on_ed25519_curve(&hash) {
        Some(hash)
    } else {
        None
    }
}

fn is_on_ed25519_curve(bytes: &[u8; 32]) -> bool {
    curve25519_dalek::edwards::CompressedEdwardsY(*bytes)
        .decompress()
        .is_some()
}

// ── Base58 ──

pub fn bs58_decode(s: &str) -> Result<[u8; 32]> {
    let bytes = bs58::decode(s)
        .into_vec()
        .with_context(|| format!("Invalid base58: {}", s))?;
    if bytes.len() != 32 {
        anyhow::bail!("Expected 32 bytes, got {} for {}", bytes.len(), s);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

// ── Compact-u16 Encoding (Solana wire format) ──

pub fn compact_u16(buf: &mut Vec<u8>, val: u16) {
    if val < 0x80 {
        buf.push(val as u8);
    } else if val < 0x4000 {
        buf.push((val & 0x7F | 0x80) as u8);
        buf.push((val >> 7) as u8);
    } else {
        buf.push((val & 0x7F | 0x80) as u8);
        buf.push(((val >> 7) & 0x7F | 0x80) as u8);
        buf.push((val >> 14) as u8);
    }
}

// ── Instruction Representation ──

pub struct TxInstruction {
    pub program_id: [u8; 32],
    pub accounts: Vec<([u8; 32], bool, bool)>, // (pubkey, is_signer, is_writable)
    pub data: Vec<u8>,
}

// ── MessageV0 Transaction Building ──

/// Build a complete signed VersionedTransaction (V0) with N instructions.
pub fn build_versioned_tx(
    wallet: &Wallet,
    instructions: &[TxInstruction],
    blockhash: &[u8; 32],
) -> Result<Vec<u8>> {
    let mut account_map: HashMap<[u8; 32], (bool, bool)> = HashMap::new();

    for ix in instructions {
        account_map.entry(ix.program_id).or_insert((false, false));
        for &(pubkey, is_signer, is_writable) in &ix.accounts {
            let entry = account_map.entry(pubkey).or_insert((false, false));
            entry.0 |= is_signer;
            entry.1 |= is_writable;
        }
    }

    // Sort: signers first, then writable, then readonly
    let mut sorted_accounts: Vec<([u8; 32], bool, bool)> = account_map
        .into_iter()
        .map(|(k, (s, w))| (k, s, w))
        .collect();
    sorted_accounts.sort_by(|a, b| {
        let a_score = (if a.1 { 4 } else { 0 }) + (if a.2 { 2 } else { 0 });
        let b_score = (if b.1 { 4 } else { 0 }) + (if b.2 { 2 } else { 0 });
        b_score.cmp(&a_score)
    });

    let account_index: HashMap<[u8; 32], u8> = sorted_accounts
        .iter()
        .enumerate()
        .map(|(i, (k, _, _))| (*k, i as u8))
        .collect();

    let num_required_signatures: u8 = sorted_accounts.iter().filter(|(_, s, _)| *s).count() as u8;
    let num_readonly_signed: u8 = sorted_accounts
        .iter()
        .filter(|(_, s, w)| *s && !*w)
        .count() as u8;
    let num_readonly_unsigned: u8 = sorted_accounts
        .iter()
        .filter(|(_, s, w)| !*s && !*w)
        .count() as u8;

    // V0 message
    let mut msg = Vec::with_capacity(2048);
    msg.push(0x80); // V0 prefix

    // Header
    msg.push(num_required_signatures);
    msg.push(num_readonly_signed);
    msg.push(num_readonly_unsigned);

    // Account keys
    compact_u16(&mut msg, sorted_accounts.len() as u16);
    for (pubkey, _, _) in &sorted_accounts {
        msg.extend_from_slice(pubkey);
    }

    // Recent blockhash
    msg.extend_from_slice(blockhash);

    // Instructions
    compact_u16(&mut msg, instructions.len() as u16);
    for ix in instructions {
        msg.push(account_index[&ix.program_id]);
        compact_u16(&mut msg, ix.accounts.len() as u16);
        for &(pubkey, _, _) in &ix.accounts {
            msg.push(account_index[&pubkey]);
        }
        compact_u16(&mut msg, ix.data.len() as u16);
        msg.extend_from_slice(&ix.data);
    }

    // Address lookup tables (none)
    compact_u16(&mut msg, 0);

    // Sign
    let signature = wallet.sign(&msg);

    // Build full VersionedTransaction
    let mut tx = Vec::with_capacity(1 + 64 + msg.len());
    compact_u16(&mut tx, 1);
    tx.extend_from_slice(&signature);
    tx.extend_from_slice(&msg);

    Ok(tx)
}

// ── Bonding Curve Math ──

/// Calculate tokens received for buying with SOL (constant product).
pub fn calc_tokens_for_sol(vsr: u64, vtr: u64, sol_lamports: u64) -> u64 {
    let new_vsr = vsr.saturating_add(sol_lamports);
    if new_vsr == 0 {
        return 0;
    }
    let k = vsr as u128 * vtr as u128;
    let new_vtr = (k / new_vsr as u128) as u64;
    vtr.saturating_sub(new_vtr)
}

/// Calculate SOL output for selling tokens (constant product).
pub fn calc_sol_output(vsr: u64, vtr: u64, token_amount: u64) -> u64 {
    let new_vtr = vtr.saturating_add(token_amount);
    if new_vtr == 0 {
        return 0;
    }
    let k = vsr as u128 * vtr as u128;
    let new_vsr = (k / new_vtr as u128) as u64;
    vsr.saturating_sub(new_vsr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs58_decode() {
        let result = bs58_decode("11111111111111111111111111111111");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0u8; 32]);
    }

    #[test]
    fn test_compact_u16() {
        let mut buf = Vec::new();
        compact_u16(&mut buf, 0);
        assert_eq!(buf, vec![0]);
        buf.clear();
        compact_u16(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
    }

    #[test]
    fn test_buy_sell_roundtrip() {
        let vsr: u64 = 50_000_000_000;
        let vtr: u64 = 900_000_000_000_000;
        let sol: u64 = 10_000_000;
        let tokens = calc_tokens_for_sol(vsr, vtr, sol);
        let sol_back = calc_sol_output(vsr + sol, vtr - tokens, tokens);
        // Integer rounding may cause ±1 lamport difference
        let diff = if sol_back > sol { sol_back - sol } else { sol - sol_back };
        assert!(diff < sol / 100, "roundtrip diff {} too large", diff);
    }
}
