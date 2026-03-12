//! PumpFun transaction parser — extracts CREATE, BUY, SELL, COMPLETE events from gRPC transactions.
//! Ported from RAMI/MOON/src/parser.rs.

/// PumpFun Program ID
pub const PUMPFUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Instruction discriminators
const CREATE_DISC: [u8; 8] = [0x18, 0x1e, 0xc8, 0x28, 0x05, 0x1c, 0x07, 0x77];
const BUY_DISC: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
const SELL_DISC: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];
const CREATE_V2_DISC: [u8; 8] = [0xd6, 0x90, 0x4c, 0xec, 0x5f, 0x8b, 0x31, 0xb4];
const MIGRATE_DISC: [u8; 8] = [0xfe, 0x94, 0xff, 0x70, 0xcf, 0x8e, 0xaa, 0xa5];

/// Parsed PumpFun event.
#[derive(Debug, Clone)]
pub enum PumpEvent {
    Create { mint: String, creator: String },
    Buy { mint: String, buyer: String, token_amount: u64 },
    Sell { mint: String, seller: String, token_amount: u64 },
    Complete { mint: String },
}

/// Lightweight instruction reference.
pub struct InstructionRef<'a> {
    pub program_id_index: u32,
    pub accounts: &'a [u8],
    pub data: &'a [u8],
}

/// Parse a gRPC transaction into PumpFun events.
pub fn parse_transaction(
    account_keys: &[Vec<u8>],
    instructions: &[InstructionRef<'_>],
) -> Vec<PumpEvent> {
    let keys: Vec<String> = account_keys
        .iter()
        .map(|k| bs58::encode(k).into_string())
        .collect();

    let pumpfun_idx = match keys.iter().position(|k| k == PUMPFUN_PROGRAM) {
        Some(idx) => idx,
        None => return vec![],
    };

    let fee_payer = keys.first().cloned().unwrap_or_default();
    let mut events = Vec::new();

    for ix in instructions {
        if ix.program_id_index as usize != pumpfun_idx {
            continue;
        }
        let data = ix.data;
        if data.len() < 8 {
            continue;
        }
        let disc = &data[..8];

        if disc == CREATE_DISC || disc == CREATE_V2_DISC {
            if let Some(&mint_idx) = ix.accounts.first() {
                if (mint_idx as usize) < keys.len() {
                    events.push(PumpEvent::Create {
                        mint: keys[mint_idx as usize].clone(),
                        creator: fee_payer.clone(),
                    });
                }
            }
        } else if disc == BUY_DISC {
            let token_amount = if data.len() >= 16 {
                u64::from_le_bytes(data[8..16].try_into().unwrap_or([0; 8]))
            } else {
                0
            };
            let mint = ix.accounts.get(2)
                .and_then(|&idx| keys.get(idx as usize))
                .cloned()
                .unwrap_or_default();
            events.push(PumpEvent::Buy { mint, buyer: fee_payer.clone(), token_amount });
        } else if disc == SELL_DISC {
            let token_amount = if data.len() >= 16 {
                u64::from_le_bytes(data[8..16].try_into().unwrap_or([0; 8]))
            } else {
                0
            };
            let mint = ix.accounts.get(2)
                .and_then(|&idx| keys.get(idx as usize))
                .cloned()
                .unwrap_or_default();
            events.push(PumpEvent::Sell { mint, seller: fee_payer.clone(), token_amount });
        } else if disc == MIGRATE_DISC {
            let mint = ix.accounts.get(2)
                .and_then(|&idx| keys.get(idx as usize))
                .cloned()
                .unwrap_or_default();
            if !mint.is_empty() {
                events.push(PumpEvent::Complete { mint });
            }
        }
    }

    events
}
