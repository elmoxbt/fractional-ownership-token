use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, MintTo},
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
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

    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn handler(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidAmount);

    let global_state_bump = ctx.accounts.global_state.bump;
    let seeds = &[GLOBAL_STATE_SEED, &[global_state_bump]];
    let signer_seeds = &[&seeds[..]];

    let mint_accounts = MintTo {
        mint: ctx.accounts.base_mint.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.global_state.to_account_info(),
    };

    token_2022::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
            signer_seeds,
        ),
        amount,
    )?;

    msg!("Minted {} tokens to {}", amount, ctx.accounts.recipient_token_account.key());

    Ok(())
}
