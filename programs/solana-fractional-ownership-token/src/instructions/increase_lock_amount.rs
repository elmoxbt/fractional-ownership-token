use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked, MintTo},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;
use crate::utils::calculate_time_multiplier;

#[derive(Accounts)]
pub struct IncreaseLockAmount<'info> {
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
        mut,
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
}

pub fn handler(ctx: Context<IncreaseLockAmount>, additional_amount: u64) -> Result<()> {
    require!(additional_amount > 0, ErrorCode::InvalidAmount);

    let current_time = Clock::get()?.unix_timestamp;
    let user_lock = &mut ctx.accounts.user_lock;

    require!(user_lock.locked_amount > 0, ErrorCode::NoExistingLock);
    require!(current_time < user_lock.unlock_time, ErrorCode::LockExpired);

    let remaining_duration = user_lock
        .unlock_time
        .checked_sub(current_time)
        .ok_or(ErrorCode::MathOverflow)?;

    let time_multiplier = calculate_time_multiplier(
        remaining_duration,
        ctx.accounts.global_state.lock_multiplier_numerator,
        ctx.accounts.global_state.lock_multiplier_denominator,
    )?;

    let additional_ve_amount = additional_amount
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
        additional_amount,
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
        additional_ve_amount,
    )?;

    user_lock.locked_amount = user_lock
        .locked_amount
        .checked_add(additional_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    user_lock.initial_ve_amount = user_lock
        .initial_ve_amount
        .checked_add(additional_ve_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    let global_state = &mut ctx.accounts.global_state;
    global_state.total_locked = global_state
        .total_locked
        .checked_add(additional_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    global_state.total_ve_supply = global_state
        .total_ve_supply
        .checked_add(additional_ve_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!("Increased lock by {} tokens", additional_amount);
    msg!("Minted {} additional veTokens", additional_ve_amount);

    Ok(())
}
