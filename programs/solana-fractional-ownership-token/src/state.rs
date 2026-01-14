use anchor_lang::prelude::*;

#[account]
pub struct GlobalState {
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub ve_mint: Pubkey,
    pub token_vault: Pubkey,
    pub fee_vault: Pubkey,
    pub total_locked: u64,
    pub total_ve_supply: u64,
    pub total_fees_deposited: u64,
    pub cumulative_fee_per_ve_token: u128, // Scaled by 1e18 for precision
    pub lock_multiplier_numerator: u64,
    pub lock_multiplier_denominator: u64,
    pub bump: u8,
}

#[account]
pub struct UserLock {
    pub user: Pubkey,
    pub locked_amount: u64,
    pub unlock_time: i64,
    pub lock_start_time: i64,
    pub initial_ve_amount: u64,
    pub fees_claimed: u64,
    pub fee_debt: u128,
    pub bump: u8,
}
