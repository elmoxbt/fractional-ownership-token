use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::constants::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<GlobalState>(),
        seeds = [GLOBAL_STATE_SEED],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = global_state,
        mint::token_program = token_program,
    )]
    pub base_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = global_state,
        mint::token_program = token_program,
    )]
    pub ve_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        token::mint = base_mint,
        token::authority = global_state,
        token::token_program = token_program,
        seeds = [TOKEN_VAULT_SEED],
        bump
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = base_mint,
        token::authority = global_state,
        token::token_program = token_program,
        seeds = [FEE_VAULT_SEED],
        bump
    )]
    pub fee_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Initialize>,
    lock_multiplier_numerator: u64,
    lock_multiplier_denominator: u64,
) -> Result<()> {
    let global_state = &mut ctx.accounts.global_state;

    global_state.authority = ctx.accounts.authority.key();
    global_state.base_mint = ctx.accounts.base_mint.key();
    global_state.ve_mint = ctx.accounts.ve_mint.key();
    global_state.token_vault = ctx.accounts.token_vault.key();
    global_state.fee_vault = ctx.accounts.fee_vault.key();
    global_state.total_locked = 0;
    global_state.total_ve_supply = 0;
    global_state.total_fees_deposited = 0;
    global_state.cumulative_fee_per_ve_token = 0;
    global_state.lock_multiplier_numerator = lock_multiplier_numerator;
    global_state.lock_multiplier_denominator = lock_multiplier_denominator;
    global_state.bump = ctx.bumps.global_state;

    msg!("Protocol initialized");
    msg!("Base mint: {}", ctx.accounts.base_mint.key());
    msg!("VeToken mint: {}", ctx.accounts.ve_mint.key());

    Ok(())
}
