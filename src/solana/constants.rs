/// Solana program addresses and constants for PumpFun + PumpSwap.
/// Consolidated from RAMI/MOON/src/rpc.rs + RAMI/GRAD/rust_monitor/src/pumpswap_buyer.rs.

// ── PumpFun (bonding curve) ──

pub const PUMP_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMP_GLOBAL_STATE: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const PUMP_FEE_RECIPIENT: &str = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM";
pub const PUMP_EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const PUMP_FEE_PROGRAM: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";

/// Fee config PDA seed
pub const PUMP_FEE_CONFIG_SEED: [u8; 32] = [
    1, 86, 224, 246, 147, 102, 90, 207,
    68, 219, 21, 104, 191, 23, 91, 170,
    81, 137, 203, 151, 245, 210, 255, 59,
    101, 93, 43, 182, 253, 109, 24, 176,
];

pub const PUMP_BUY_DISCRIMINATOR: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
pub const PUMP_SELL_DISCRIMINATOR: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

// ── PumpSwap (AMM — graduated tokens) ──

pub const PUMPSWAP_PROGRAM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMPSWAP_GLOBAL_CONFIG: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
pub const PUMPSWAP_EVENT_AUTHORITY: &str = "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR";
pub const PUMPSWAP_FEE_CONFIG: &str = "5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx";
pub const PUMPSWAP_FEE_PROGRAM: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";
pub const PUMPSWAP_GLOBAL_VOL_ACC: &str = "C2aFPdENg4A2HQsmrd5rTw5TaYBX5Ku887cWjbFKtZpw";

/// sha256("global:buy_exact_quote_in")[:8]
pub const PUMPSWAP_BUY_DISCRIMINATOR: [u8; 8] = [0xC6, 0x2E, 0x15, 0x52, 0xB4, 0xD9, 0xE8, 0x70];
/// sha256("global:sell")[:8]
pub const PUMPSWAP_SELL_DISCRIMINATOR: [u8; 8] = [0x33, 0xE6, 0x85, 0xA4, 0x01, 0x7F, 0x83, 0xAD];

/// Protocol fee recipients for normal pools
pub const PROTOCOL_FEE_RECIPIENTS: &[&str] = &[
    "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV",
    "7VtfL8fvgNfhz17qKRMjzQEXgbdpnHHHQRh54R9jP2RJ",
    "7hTckgnGnLQR6sdH7YkqFTAA7VwTfYFaZ6EhEsU3saCX",
    "9rPYyANsfQZw3DnDmKE3YCQF5E8oD89UXoHn9JFEhJUz",
    "AVmoTthdrX6tKt4nDjco2D775W2YK3sDhxPcMmzUAmTY",
    "FWsW1xNtWscwNmKv6wVsU1iTzRN6wmmk3MjxRP5tT7hz",
    "G5UZAVbAf46s7cKWoyKu8kYTip9DGTpbLZ2qa9Aq69dP",
    "JCRGumoE9Qi5BBgULTgdgTLjSgkCMSbF62ZZfGs84JeU",
];

/// Protocol fee recipients for mayhem pools
pub const MAYHEM_FEE_RECIPIENTS: &[&str] = &[
    "GesfTA3X2arioaHp8bbKdjG9vJtskViWACZoYvxp4twS",
    "4budycTjhs9fD6xw62VBducVTNgMgJJ5BgtKq7mAZwn6",
    "8SBKzEQU4nLSzcwF4a74F2iaUDQyTfjGndn6qUWBnrpR",
    "4UQeTP1T39KZ9Sfxzo3WR5skgsaP6NZa87BAkuazLEKH",
    "8sNeir4QsLsJdYpc9RZacohhK1Y5FLU3nC5LXgYB4aa6",
    "Fh9HmeLNUMVCvejxCtCL2DbYaRyBFVJ5xrWkLnMH6fdk",
    "463MEnMeGyJekNZFQSTUABBEbLnvMTALbT6ZmsxAbAdq",
    "6AUH3WEHucYZyC61hqpqYUWVto5qA5hjHuNQ32GNnNxA",
];

// ── Pool data offsets ──

pub const POOL_COIN_CREATOR_OFFSET: usize = 211;
pub const POOL_IS_MAYHEM_OFFSET: usize = 243;
pub const POOL_IS_CASHBACK_OFFSET: usize = 244;

// ── Common Solana programs ──

pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const ATA_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const COMPUTE_BUDGET_PROGRAM: &str = "ComputeBudget111111111111111111111111111111";
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// ── Compute Budget defaults ──

pub const COMPUTE_UNIT_LIMIT: u32 = 300_000;
pub const COMPUTE_UNIT_PRICE_MICRO_LAMPORTS: u64 = 1_667; // ~500K lamports / 300K CU

pub const PUMPSWAP_COMPUTE_UNIT_LIMIT: u32 = 400_000;
pub const PUMPSWAP_COMPUTE_UNIT_PRICE_MICRO_LAMPORTS: u64 = 150_000 * 1_000_000 / 400_000;

// ── Slippage ──

pub const BUY_SLIPPAGE_BPS: u64 = 2000; // 20%
pub const SELL_SLIPPAGE_BPS: u64 = 5000; // 50%
