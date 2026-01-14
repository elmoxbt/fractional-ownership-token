use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, MintTo},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;
use crate::utils::calculate_time_multiplier;

#[derive(Accounts)]
pub struct IncreaseLockDuration<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [USER_LOCK_SEED, user.key().as_ref()],
        bump = user_lock.bump
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
        constraint = ve_mint.key() == global_state.ve_mint
    )]
    pub ve_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = ve_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_ve_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<IncreaseLockDuration>, additional_duration: i64) -> Result<()> {
    require!(additional_duration > 0, ErrorCode::InvalidLockDuration);

    let current_time = Clock::get()?.unix_timestamp;
    let user_lock = &mut ctx.accounts.user_lock;

    require!(user_lock.locked_amount > 0, ErrorCode::NoExistingLock);
    require!(current_time < user_lock.unlock_time, ErrorCode::LockExpired);

    let new_unlock_time = user_lock
        .unlock_time
        .checked_add(additional_duration)
        .ok_or(ErrorCode::MathOverflow)?;

    let new_total_duration = new_unlock_time
        .checked_sub(user_lock.lock_start_time)
        .ok_or(ErrorCode::MathOverflow)?;

    require!(
        new_total_duration <= MAX_LOCK_DURATION,
        ErrorCode::InvalidLockDuration
    );

    let time_multiplier = calculate_time_multiplier(
        new_total_duration,
        ctx.accounts.global_state.lock_multiplier_numerator,
        ctx.accounts.global_state.lock_multiplier_denominator,
    )?;

    let new_ve_amount = user_lock
        .locked_amount
        .checked_mul(time_multiplier)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(ctx.accounts.global_state.lock_multiplier_denominator)
        .ok_or(ErrorCode::MathOverflow)?;

    if new_ve_amount > user_lock.initial_ve_amount {
        let additional_ve_amount = new_ve_amount
            .checked_sub(user_lock.initial_ve_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        let global_state_bump = ctx.accounts.global_state.bump;
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
            additional_ve_amount,
        )?;

        let global_state = &mut ctx.accounts.global_state;
        global_state.total_ve_supply = global_state
            .total_ve_supply
            .checked_add(additional_ve_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        user_lock.initial_ve_amount = new_ve_amount;

        msg!("Minted {} additional veTokens", additional_ve_amount);
    }

    user_lock.unlock_time = new_unlock_time;

    msg!("Extended lock until {}", new_unlock_time);

    Ok(())
}
