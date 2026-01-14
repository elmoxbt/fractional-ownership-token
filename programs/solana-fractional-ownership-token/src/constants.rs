pub const MIN_LOCK_DURATION: i64 = 7 * 24 * 60 * 60;
pub const MAX_LOCK_DURATION: i64 = 4 * 365 * 24 * 60 * 60;
pub const MAX_LOCK_MULTIPLIER: u64 = 4;

pub const GLOBAL_STATE_SEED: &[u8] = b"global-state";
pub const USER_LOCK_SEED: &[u8] = b"user-lock";
pub const FEE_VAULT_SEED: &[u8] = b"fee-vault";
pub const TOKEN_VAULT_SEED: &[u8] = b"token-vault";
