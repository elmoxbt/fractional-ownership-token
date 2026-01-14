use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;
use crate::utils::calculate_current_ve_balance;

#[derive(Accounts)]
pub struct ClaimFees<'info> {
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
        associated_token::mint = base_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [FEE_VAULT_SEED],
        bump,
        constraint = fee_vault.key() == global_state.fee_vault
    )]
    pub fee_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<ClaimFees>) -> Result<()> {
    let user_lock = &mut ctx.accounts.user_lock;
    let current_time = Clock::get()?.unix_timestamp;

    require!(user_lock.locked_amount > 0, ErrorCode::NoExistingLock);

    let current_ve_balance = calculate_current_ve_balance(
        user_lock.initial_ve_amount,
        user_lock.lock_start_time,
        user_lock.unlock_time,
        current_time,
    )?;

    require!(current_ve_balance > 0, ErrorCode::NoVotingPower);

    let cumulative_fee_per_ve_token = ctx.accounts.global_state.cumulative_fee_per_ve_token;
    let pending_fees = (current_ve_balance as u128)
        .checked_mul(
            cumulative_fee_per_ve_token
                .checked_sub(user_lock.fee_debt)
                .ok_or(ErrorCode::MathOverflow)?
        )
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(1_000_000_000_000_000_000)
        .ok_or(ErrorCode::MathOverflow)? as u64;

    require!(pending_fees > 0, ErrorCode::NoFeesToClaim);

    let global_state_bump = ctx.accounts.global_state.bump;
    let seeds = &[GLOBAL_STATE_SEED, &[global_state_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.fee_vault.to_account_info(),
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
        pending_fees,
        ctx.accounts.base_mint.decimals,
    )?;

    user_lock.fee_debt = cumulative_fee_per_ve_token;
    user_lock.fees_claimed = user_lock.fees_claimed
        .checked_add(pending_fees)
        .ok_or(ErrorCode::MathOverflow)?;

    msg!("Claimed {} fees", pending_fees);
    msg!("veBalance: {}", current_ve_balance);

    Ok(())
}
