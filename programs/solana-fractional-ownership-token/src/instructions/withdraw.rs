use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked, Burn},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

pub fn handler(ctx: Context<Withdraw>) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    let user_lock = &mut ctx.accounts.user_lock;

    require!(user_lock.locked_amount > 0, ErrorCode::NoExistingLock);
    require!(current_time >= user_lock.unlock_time, ErrorCode::LockNotExpired);

    let amount = user_lock.locked_amount;
    let ve_amount = user_lock.initial_ve_amount;
    let global_state_bump = ctx.accounts.global_state.bump;

    let burn_accounts = Burn {
        mint: ctx.accounts.ve_mint.to_account_info(),
        from: ctx.accounts.user_ve_token_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    token_2022::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            burn_accounts,
        ),
        ve_amount,
    )?;

    let seeds = &[GLOBAL_STATE_SEED, &[global_state_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.token_vault.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.global_state.to_account_info(),
    };

    token_2022::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        ),
        amount,
        ctx.accounts.base_mint.decimals,
    )?;

    let global_state = &mut ctx.accounts.global_state;
    global_state.total_locked = global_state
        .total_locked
        .checked_sub(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    global_state.total_ve_supply = global_state
        .total_ve_supply
        .checked_sub(ve_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    user_lock.locked_amount = 0;
    user_lock.unlock_time = 0;
    user_lock.lock_start_time = 0;
    user_lock.initial_ve_amount = 0;

    msg!("Withdrew {} tokens", amount);
    msg!("Burned {} veTokens", ve_amount);

    Ok(())
}
