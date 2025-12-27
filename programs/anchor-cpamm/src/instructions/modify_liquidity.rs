use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Pool, Position, TickArray};
use crate::errors::ClammError;
use crate::math::{
    tick_math::sqrt_price_from_tick_index,
    liquidity_math::get_amounts_for_liquidity,
    fee_math::get_fee_growth_inside,
};

#[derive(Accounts)]
pub struct ModifyLiquidity<'info> {
    #[account(mut)]
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
        mut,
        constraint = tick_array_lower.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_lower: AccountLoader<'info, TickArray>,

    #[account(
        mut,
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

    #[account(mut)]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ModifyLiquidity<'info> {
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

    pub fn update_position_fees(&mut self, fee_growth_inside_x: u128, fee_growth_inside_y: u128) {
        self.position.update_fees_owed(fee_growth_inside_x, fee_growth_inside_y);
    }

    pub fn calculate_token_amounts(&self, liquidity: u128, round_up: bool) -> Result<(u64, u64)> {
        let sqrt_price_lower = sqrt_price_from_tick_index(self.position.tick_lower_index)?;
        let sqrt_price_upper = sqrt_price_from_tick_index(self.position.tick_upper_index)?;

        get_amounts_for_liquidity(
            self.pool.sqrt_price,
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity,
            round_up,
        )
    }

    pub fn update_tick(
        &self,
        tick_array: &AccountLoader<'info, TickArray>,
        tick_index: i32,
        liquidity_delta: i128,
        is_upper: bool,
    ) -> Result<()> {
        let mut tick_array = tick_array.load_mut()?;
        let tick = tick_array
            .get_tick_mut(tick_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;
        tick.update(liquidity_delta, is_upper);
        Ok(())
    }

    pub fn add_pool_liquidity(&mut self, liquidity: u128) -> Result<()> {
        if self.pool.is_price_in_range(self.position.tick_lower_index, self.position.tick_upper_index) {
            self.pool.liquidity = self.pool.liquidity
                .checked_add(liquidity)
                .ok_or(ClammError::LiquidityOverflow)?;
        }
        Ok(())
    }

    pub fn sub_pool_liquidity(&mut self, liquidity: u128) -> Result<()> {
        if self.pool.is_price_in_range(self.position.tick_lower_index, self.position.tick_upper_index) {
            self.pool.liquidity = self.pool.liquidity
                .checked_sub(liquidity)
                .ok_or(ClammError::LiquidityUnderflow)?;
        }
        Ok(())
    }

    pub fn add_position_liquidity(&mut self, liquidity: u128) -> Result<()> {
        self.position.liquidity = self.position.liquidity
            .checked_add(liquidity)
            .ok_or(ClammError::LiquidityOverflow)?;
        Ok(())
    }

    pub fn sub_position_liquidity(&mut self, liquidity: u128) -> Result<()> {
        self.position.liquidity = self.position.liquidity
            .checked_sub(liquidity)
            .ok_or(ClammError::LiquidityUnderflow)?;
        Ok(())
    }

    pub fn deposit_tokens(&self, amount_x: u64, amount_y: u64) -> Result<()> {
        if amount_x > 0 {
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_token_x.to_account_info(),
                        to: self.token_vault_x.to_account_info(),
                        authority: self.owner.to_account_info(),
                    },
                ),
                amount_x,
            )?;
        }
        if amount_y > 0 {
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_token_y.to_account_info(),
                        to: self.token_vault_y.to_account_info(),
                        authority: self.owner.to_account_info(),
                    },
                ),
                amount_y,
            )?;
        }
        Ok(())
    }

    pub fn withdraw_tokens(&self, amount_x: u64, amount_y: u64) -> Result<()> {
        let seeds = &[
            b"pool".as_ref(),
            self.pool.token_mint_x.as_ref(),
            self.pool.token_mint_y.as_ref(),
            &[self.pool.bump],
        ];
        let signer = &[&seeds[..]];

        if amount_x > 0 {
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
                amount_x,
            )?;
        }
        if amount_y > 0 {
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
                amount_y,
            )?;
        }
        Ok(())
    }
}

/// Add liquidity to an existing position
pub fn add_liquidity_handler(ctx: Context<ModifyLiquidity>, liquidity_delta: u128) -> Result<()> {
    // Calculate and update fees
    let (fee_growth_inside_x, fee_growth_inside_y) = ctx.accounts.get_fee_growth_inside()?;
    ctx.accounts.update_position_fees(fee_growth_inside_x, fee_growth_inside_y);

    // Calculate token amounts (round up for deposits)
    let (amount_x, amount_y) = ctx.accounts.calculate_token_amounts(liquidity_delta, true)?;

    // Update ticks
    ctx.accounts.update_tick(
        &ctx.accounts.tick_array_lower,
        ctx.accounts.position.tick_lower_index,
        liquidity_delta as i128,
        false,
    )?;
    ctx.accounts.update_tick(
        &ctx.accounts.tick_array_upper,
        ctx.accounts.position.tick_upper_index,
        liquidity_delta as i128,
        true,
    )?;

    // Update pool and position liquidity
    ctx.accounts.add_pool_liquidity(liquidity_delta)?;
    ctx.accounts.add_position_liquidity(liquidity_delta)?;

    // Deposit tokens from user to vault
    ctx.accounts.deposit_tokens(amount_x, amount_y)?;

    Ok(())
}

/// Remove liquidity from an existing position
pub fn remove_liquidity_handler(ctx: Context<ModifyLiquidity>, liquidity_delta: u128) -> Result<()> {
    require!(
        liquidity_delta <= ctx.accounts.position.liquidity,
        ClammError::LiquidityUnderflow
    );

    // Calculate and update fees
    let (fee_growth_inside_x, fee_growth_inside_y) = ctx.accounts.get_fee_growth_inside()?;
    ctx.accounts.update_position_fees(fee_growth_inside_x, fee_growth_inside_y);

    // Calculate token amounts (round down for withdrawals)
    let (amount_x, amount_y) = ctx.accounts.calculate_token_amounts(liquidity_delta, false)?;

    // Update ticks (negative delta for removal)
    ctx.accounts.update_tick(
        &ctx.accounts.tick_array_lower,
        ctx.accounts.position.tick_lower_index,
        -(liquidity_delta as i128),
        false,
    )?;
    ctx.accounts.update_tick(
        &ctx.accounts.tick_array_upper,
        ctx.accounts.position.tick_upper_index,
        -(liquidity_delta as i128),
        true,
    )?;

    // Update pool and position liquidity
    ctx.accounts.sub_pool_liquidity(liquidity_delta)?;
    ctx.accounts.sub_position_liquidity(liquidity_delta)?;

    // Withdraw tokens from vault to user
    ctx.accounts.withdraw_tokens(amount_x, amount_y)?;

    Ok(())
}
