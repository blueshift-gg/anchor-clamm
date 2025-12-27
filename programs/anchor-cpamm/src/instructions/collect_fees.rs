use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Pool, Position, TickArray};
use crate::errors::ClammError;
use crate::math::fee_math::get_fee_growth_inside;

#[derive(Accounts)]
pub struct CollectFees<'info> {
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = position.pool == pool.key()
    )]
    pub position: Account<'info, Position>,

    /// Token account holding the position NFT (proves ownership)
    #[account(
        constraint = position_token_account.mint == position.position_mint,
        constraint = position_token_account.amount > 0,
        constraint = position_token_account.owner == owner.key()
    )]
    pub position_token_account: Account<'info, TokenAccount>,

    #[account(
        constraint = tick_array_lower.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    #[account(
        constraint = tick_array_upper.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    #[account(
        mut,
        constraint = token_vault_x.mint == pool.token_mint_x,
        constraint = token_vault_x.owner == pool.key()
    )]
    pub token_vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_vault_y.mint == pool.token_mint_y,
        constraint = token_vault_y.owner == pool.key()
    )]
    pub token_vault_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_x.mint == pool.token_mint_x
    )]
    pub user_token_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_y.mint == pool.token_mint_y
    )]
    pub user_token_y: Account<'info, TokenAccount>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> CollectFees<'info> {
    pub fn get_fee_growth_inside(&self) -> Result<(u128, u128)> {
        let tick_array_lower = self.tick_array_lower.load()?;
        let tick_array_upper = self.tick_array_upper.load()?;

        let tick_lower = tick_array_lower
            .get_tick(self.position.tick_lower_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;
        let tick_upper = tick_array_upper
            .get_tick(self.position.tick_upper_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;

        Ok(get_fee_growth_inside(
            tick_lower,
            tick_upper,
            self.position.tick_lower_index,
            self.position.tick_upper_index,
            self.pool.tick_current_index,
            self.pool.fee_growth_global_x,
            self.pool.fee_growth_global_y,
        ))
    }

    pub fn update_and_reset_fees(&mut self, fee_growth_inside_x: u128, fee_growth_inside_y: u128) -> (u64, u64) {
        self.position.update_fees_owed(fee_growth_inside_x, fee_growth_inside_y);

        let fees_x = self.position.tokens_owed_x;
        let fees_y = self.position.tokens_owed_y;

        self.position.tokens_owed_x = 0;
        self.position.tokens_owed_y = 0;

        (fees_x, fees_y)
    }

    pub fn transfer_fees(&self, fees_x: u64, fees_y: u64) -> Result<()> {
        let seeds = &[
            b"pool".as_ref(),
            self.pool.token_mint_x.as_ref(),
            self.pool.token_mint_y.as_ref(),
            &[self.pool.bump],
        ];
        let signer = &[&seeds[..]];

        if fees_x > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_vault_x.to_account_info(),
                        to: self.user_token_x.to_account_info(),
                        authority: self.pool.to_account_info(),
                    },
                    signer,
                ),
                fees_x,
            )?;
        }

        if fees_y > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_vault_y.to_account_info(),
                        to: self.user_token_y.to_account_info(),
                        authority: self.pool.to_account_info(),
                    },
                    signer,
                ),
                fees_y,
            )?;
        }

        Ok(())
    }
}

/// Collect accumulated fees from a position
pub fn collect_fees_handler(ctx: Context<CollectFees>) -> Result<()> {
    // Calculate the current Fees
    let (fee_growth_inside_x, fee_growth_inside_y) = ctx.accounts.get_fee_growth_inside()?;
    
    // Update and reset the fees collected in the position
    let (fees_x, fees_y) = ctx.accounts.update_and_reset_fees(fee_growth_inside_x, fee_growth_inside_y);
    
    // Transfer the Fees collected
    ctx.accounts.transfer_fees(fees_x, fees_y)
}
