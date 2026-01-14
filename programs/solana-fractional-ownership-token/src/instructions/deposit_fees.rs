use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;

#[derive(Accounts)]
pub struct DepositFees<'info> {
    #[account(
        mut,
        constraint = authority.key() == global_state.authority
    )]
    pub authority: Signer<'info>,

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
        associated_token::authority = authority,
        associated_token::token_program = token_program
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [FEE_VAULT_SEED],
        bump,
        constraint = fee_vault.key() == global_state.fee_vault
    )]
    pub fee_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<DepositFees>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidAmount);

    let global_state = &mut ctx.accounts.global_state;

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.authority_token_account.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: ctx.accounts.fee_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    token_2022::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        ),
        amount,
        ctx.accounts.base_mint.decimals,
    )?;

    global_state.total_fees_deposited = global_state
        .total_fees_deposited
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    if global_state.total_ve_supply > 0 {
        let fee_per_ve_token = (amount as u128)
            .checked_mul(1_000_000_000_000_000_000)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(global_state.total_ve_supply as u128)
            .ok_or(ErrorCode::MathOverflow)?;

        global_state.cumulative_fee_per_ve_token = global_state
            .cumulative_fee_per_ve_token
            .checked_add(fee_per_ve_token)
            .ok_or(ErrorCode::MathOverflow)?;
    }

    msg!("Deposited {} fees to vault", amount);

    Ok(())
}
