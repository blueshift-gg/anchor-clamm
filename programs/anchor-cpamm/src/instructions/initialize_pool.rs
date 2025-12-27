use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use crate::state::Pool;
use crate::errors::ClammError;
use crate::math::tick_math::tick_index_from_sqrt_price;
use crate::math::q64_64::sqrt_price_from_amounts;
use crate::constants::{MIN_SQRT_PRICE, MAX_SQRT_PRICE, MAX_FEE_RATE};

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Pool PDA
    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [
            b"pool",
            token_mint_x.key().min(token_mint_y.key()).as_ref(),                                                                                                               
            token_mint_x.key().max(token_mint_y.key()).as_ref()
        ],
        bump,
    )]
    pub pool: Account<'info, Pool>,

    /// Token X mint
    pub token_mint_x: Account<'info, Mint>,

    /// Token Y mint
    pub token_mint_y: Account<'info, Mint>,

    /// Vault for token X
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_mint_x,
        associated_token::authority = pool,
    )]
    pub token_vault_x: Account<'info, TokenAccount>,

    /// Vault for token Y
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_mint_y,
        associated_token::authority = pool,
    )]
    pub token_vault_y: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> InitializePool<'info> {
    pub fn validate(
        &self,
        initial_sqrt_price: u128, 
        fee_rate: u16, 
        tick_spacing: u16
    ) -> Result<()> {
        // Validate sqrt_price is within bounds
        require!(
            initial_sqrt_price >= MIN_SQRT_PRICE && initial_sqrt_price <= MAX_SQRT_PRICE,
            ClammError::InvalidSqrtPrice
        );

        // Validate fee rate
        require!(fee_rate <= MAX_FEE_RATE as u16, ClammError::InvalidFeeRate);

        // Validate tick spacing
        require!(tick_spacing > 0, ClammError::InvalidTickSpacing);

        Ok(())
    }

    pub fn initialize_pool(
        &mut self,
        initial_sqrt_price: u128,
        tick_current_index: i32,
        fee_rate: u16,
        tick_spacing: u16,
        bump: u8,
    ) {
        self.pool.set_inner(Pool {
            token_mint_x: self.token_mint_x.key(),
            token_mint_y: self.token_mint_y.key(),
            sqrt_price: initial_sqrt_price,
            tick_current_index,
            liquidity: 0,
            fee_rate,
            tick_spacing,
            fee_growth_global_x: 0,
            fee_growth_global_y: 0,
            bump,
        });
    }
}

/// Initialize a new pool with a starting price derived from token amounts.
/// Price is computed as sqrt(amount_y / amount_x) in Q64.64 format.
pub fn initialize_pool_handler(
    ctx: Context<InitializePool>,
    amount_x: u64,
    amount_y: u64,
    fee_rate: u16,
    tick_spacing: u16,
) -> Result<()> {
    // Compute sqrt_price from amounts: sqrt(amount_y / amount_x) in Q64.64
    let initial_sqrt_price = sqrt_price_from_amounts(amount_x, amount_y)?;
    
    // Validate parameters
    ctx.accounts.validate(initial_sqrt_price, fee_rate, tick_spacing)?;

    // Calculate initial tick from sqrt_price
    let tick_current_index = tick_index_from_sqrt_price(initial_sqrt_price)?;

    // Initialize pool state
    ctx.accounts.initialize_pool(
        initial_sqrt_price,
        tick_current_index,
        fee_rate,
        tick_spacing,
        ctx.bumps.pool,
    );

    Ok(())
}
