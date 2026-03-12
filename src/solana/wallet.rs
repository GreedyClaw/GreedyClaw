//! Solana keypair loading and Ed25519 signing.
//! Adapted from RAMI/MOON/src/wallet.rs — production-proven.

use anyhow::{Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use std::sync::Arc;

/// Wallet wraps a Solana keypair for transaction signing.
#[derive(Clone)]
pub struct Wallet {
    signing_key: Arc<SigningKey>,
    pubkey_bytes: [u8; 32],
    pubkey_b58: String,
}

impl Wallet {
    /// Load keypair from Solana CLI JSON format: [u8; 64] as JSON array.
    /// First 32 bytes = secret key, last 32 = public key.
    pub fn from_file(path: &str) -> Result<Self> {
        let json_str = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read keypair file: {}", path))?;
        let bytes: Vec<u8> = serde_json::from_str(&json_str)
            .with_context(|| "Failed to parse keypair JSON (expected [u8; 64] array)")?;
        if bytes.len() != 64 {
            anyhow::bail!("Keypair must be 64 bytes, got {}", bytes.len());
        }
        let secret: [u8; 32] = bytes[..32].try_into()?;
        let signing_key = SigningKey::from_bytes(&secret);
        let pubkey_bytes = signing_key.verifying_key().to_bytes();
        let pubkey_b58 = bs58::encode(&pubkey_bytes).into_string();

        tracing::info!("[WALLET] Loaded keypair: {}...", &pubkey_b58[..12]);
        Ok(Self {
            signing_key: Arc::new(signing_key),
            pubkey_bytes,
            pubkey_b58,
        })
    }

    pub fn pubkey(&self) -> &[u8; 32] {
        &self.pubkey_bytes
    }

    pub fn pubkey_b58(&self) -> &str {
        &self.pubkey_b58
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_roundtrip() {
        let secret = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();
        let message = b"test message";
        let sig = signing_key.sign(message);
        assert!(verifying_key.verify_strict(message, &sig).is_ok());
    }
}
