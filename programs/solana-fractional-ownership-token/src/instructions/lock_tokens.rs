use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked, MintTo},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;
use crate::utils::calculate_time_multiplier;

#[derive(Accounts)]
pub struct LockTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + std::mem::size_of::<UserLock>(),
        seeds = [USER_LOCK_SEED, user.key().as_ref()],
        bump
    )]
    pub user_lock: Account<'info, UserLock>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        constraint = base_mint.key() == global_state.base_mint
    )]
    pub base_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = ve_mint.key() == global_state.ve_mint
    )]
    pub ve_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = ve_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_ve_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED],
        bump,
        constraint = token_vault.key() == global_state.token_vault
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<LockTokens>, amount: u64, lock_duration: i64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidAmount);
    require!(
        lock_duration >= MIN_LOCK_DURATION && lock_duration <= MAX_LOCK_DURATION,
        ErrorCode::InvalidLockDuration
    );

    let current_time = Clock::get()?.unix_timestamp;

    let user_lock = &mut ctx.accounts.user_lock;
    let is_new_lock = user_lock.locked_amount == 0;

    if is_new_lock {
        user_lock.user = ctx.accounts.user.key();
        user_lock.locked_amount = 0;
        user_lock.unlock_time = 0;
        user_lock.lock_start_time = 0;
        user_lock.initial_ve_amount = 0;
        user_lock.fees_claimed = 0;
        user_lock.fee_debt = ctx.accounts.global_state.cumulative_fee_per_ve_token;
        user_lock.bump = ctx.bumps.user_lock;
    }

    let time_multiplier = calculate_time_multiplier(
        lock_duration,
        ctx.accounts.global_state.lock_multiplier_numerator,
        ctx.accounts.global_state.lock_multiplier_denominator,
    )?;

    let new_ve_amount = amount
        .checked_mul(time_multiplier)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(ctx.accounts.global_state.lock_multiplier_denominator)
        .ok_or(ErrorCode::MathOverflow)?;

    let global_state_bump = ctx.accounts.global_state.bump;

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.user_token_account.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    token_2022::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        ),
        amount,
        ctx.accounts.base_mint.decimals,
    )?;

    let seeds = &[GLOBAL_STATE_SEED, &[global_state_bump]];
    let signer_seeds = &[&seeds[..]];

    let mint_accounts = MintTo {
        mint: ctx.accounts.ve_mint.to_account_info(),
        to: ctx.accounts.user_ve_token_account.to_account_info(),
        authority: ctx.accounts.global_state.to_account_info(),
    };

    token_2022::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
            signer_seeds,
        ),
        new_ve_amount,
    )?;

    let new_unlock_time = current_time
        .checked_add(lock_duration)
        .ok_or(ErrorCode::MathOverflow)?;

    // Calculate weighted average unlock time if adding to existing lock
    let final_unlock_time = if is_new_lock {
        new_unlock_time
    } else {
        // Use u128 to avoid overflow when multiplying amounts by timestamps
        let old_weight = (user_lock.locked_amount as u128)
            .checked_mul(user_lock.unlock_time as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        let new_weight = (amount as u128)
            .checked_mul(new_unlock_time as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        let total_weight = old_weight
            .checked_add(new_weight)
            .ok_or(ErrorCode::MathOverflow)?;
        let new_total = (user_lock.locked_amount as u128)
            .checked_add(amount as u128)
            .ok_or(ErrorCode::MathOverflow)?;

        (total_weight / new_total) as i64
    };

    // Update user lock state
    user_lock.locked_amount = user_lock.locked_amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    user_lock.unlock_time = final_unlock_time;
    user_lock.lock_start_time = current_time;
    user_lock.initial_ve_amount = user_lock.initial_ve_amount
        .checked_add(new_ve_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    let global_state = &mut ctx.accounts.global_state;
    global_state.total_locked = global_state
        .total_locked
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    global_state.total_ve_supply = global_state
        .total_ve_supply
        .checked_add(new_ve_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!("Locked {} tokens until {}", amount, final_unlock_time);
    msg!("Minted {} veTokens", new_ve_amount);

    Ok(())
}
