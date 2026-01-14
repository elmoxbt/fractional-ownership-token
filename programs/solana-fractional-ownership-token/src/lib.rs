use anchor_lang::prelude::*;

declare_id!("5xjnSTgkKABxfbBz5wtfWb2ye17piZo7ad5UBFuFybzQ");

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

#[program]
pub mod solana_fractional_ownership_token {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        lock_multiplier_numerator: u64,
        lock_multiplier_denominator: u64,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, lock_multiplier_numerator, lock_multiplier_denominator)
    }

    pub fn lock_tokens(ctx: Context<LockTokens>, amount: u64, lock_duration: i64) -> Result<()> {
        instructions::lock_tokens::handler(ctx, amount, lock_duration)
    }

    pub fn increase_lock_amount(ctx: Context<IncreaseLockAmount>, additional_amount: u64) -> Result<()> {
        instructions::increase_lock_amount::handler(ctx, additional_amount)
    }

    pub fn increase_lock_duration(ctx: Context<IncreaseLockDuration>, additional_duration: i64) -> Result<()> {
        instructions::increase_lock_duration::handler(ctx, additional_duration)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        instructions::withdraw::handler(ctx)
    }

    pub fn deposit_fees(ctx: Context<DepositFees>, amount: u64) -> Result<()> {
        instructions::deposit_fees::handler(ctx, amount)
    }

    pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
        instructions::claim_fees::handler(ctx)
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        instructions::mint_tokens::handler(ctx, amount)
    }
}
