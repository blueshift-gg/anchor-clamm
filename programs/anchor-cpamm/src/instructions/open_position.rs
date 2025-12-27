use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, SetAuthority, Token, TokenAccount, Transfer},
};
use crate::state::{Pool, Position, TickArray};
use crate::errors::ClammError;
use crate::math::{
    tick_math::{check_tick, sqrt_price_from_tick_index},
    liquidity_math::get_amounts_for_liquidity,
    fee_math::get_fee_growth_inside,
};

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub pool: Account<'info, Pool>,

    /// Position PDA
    #[account(
        init,
        payer = payer,
        space = Position::LEN,
        seeds = [b"position", position_mint.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    /// NFT mint for this position
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = pool,
    )]
    pub position_mint: Account<'info, Mint>,

    /// Owner's token account for the position NFT
    #[account(
        init,
        payer = payer,
        associated_token::mint = position_mint,
        associated_token::authority = owner,
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

    /// CHECK: The owner of the position NFT (can be any account)
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> OpenPosition<'info> {
    pub fn validate(&self, tick_lower_index: i32, tick_upper_index: i32) -> Result<()> {
        // Check if all the ticks are valid
        require!(tick_lower_index < tick_upper_index, ClammError::InvalidTickRange);

        check_tick(tick_lower_index, self.pool.tick_spacing)?;
        check_tick(tick_upper_index, self.pool.tick_spacing)
    }

    pub fn update_tick(
        &self,
        tick_array: &AccountLoader<'info, TickArray>,
        tick_index: i32,
        liquidity: u128,
        is_upper: bool,
    ) -> Result<()> {
        // Deserialize the Tick
        let mut tick_array = tick_array.load_mut()?;
        let tick = tick_array
            .get_tick_mut(tick_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;

        // Check if the Tick is initialized. If not, initialize it
        if !tick.is_initialized() {
            tick.initialize(
                tick_index,
                self.pool.tick_current_index,
                self.pool.fee_growth_global_x,
                self.pool.fee_growth_global_y,
            );
        }

        // 
        tick.update(liquidity as i128, is_upper);

        Ok(())
    }

    pub fn get_fee_growth_inside(
        &self,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<(u128, u128)> {
        let tick_array_lower = self.tick_array_lower.load()?;
        let tick_array_upper = self.tick_array_upper.load()?;

        let tick_lower = tick_array_lower
            .get_tick(tick_lower_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;
        let tick_upper = tick_array_upper
            .get_tick(tick_upper_index, self.pool.tick_spacing)
            .ok_or(ClammError::TickNotInArray)?;

        Ok(get_fee_growth_inside(
            tick_lower,
            tick_upper,
            tick_lower_index,
            tick_upper_index,
            self.pool.tick_current_index,
            self.pool.fee_growth_global_x,
            self.pool.fee_growth_global_y,
        ))
    }

    pub fn initialize_position(
        &mut self,
        tick_lower_index: i32,
        tick_upper_index: i32,
        liquidity: u128,
        fee_growth_inside_x: u128,
        fee_growth_inside_y: u128,
    ) {
        self.position.set_inner(Position {
            pool: self.pool.key(),
            position_mint: self.position_mint.key(),
            tick_lower_index,
            tick_upper_index,
            liquidity,
            fee_growth_inside_last_x: fee_growth_inside_x,
            fee_growth_inside_last_y: fee_growth_inside_y,
            tokens_owed_x: 0,
            tokens_owed_y: 0,
        });
    }

    /// Mint the position NFT to the owner and remove mint authority
    pub fn mint_position_nft(&self) -> Result<()> {
        let pool = &self.pool;
        let seeds = &[
            b"pool".as_ref(),
            pool.token_mint_x.as_ref(),
            pool.token_mint_y.as_ref(),
            &[pool.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Mint 1 NFT to owner
        token::mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.position_mint.to_account_info(),
                    to: self.position_token_account.to_account_info(),
                    authority: self.pool.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        token::set_authority(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(), 
                SetAuthority {
                    current_authority: self.pool.to_account_info(),
                    account_or_mint: self.position_mint.to_account_info()
                }, 
                signer_seeds
            ), 
            token::spl_token::instruction::AuthorityType::MintTokens, 
            None
        )

        // We could add metadata to it but we're not going to do it
        
    }

    pub fn deposit_tokens(&self, amount_x: u64, amount_y: u64) -> Result<()> {
        if amount_x > 0 {
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_token_x.to_account_info(),
                        to: self.token_vault_x.to_account_info(),
                        authority: self.payer.to_account_info(),
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
                        authority: self.payer.to_account_info(),
                    },
                ),
                amount_y,
            )?;
        }

        Ok(())
    }
}

/// Open a new liquidity position in a price range [tick_lower, tick_upper).
/// Mints an NFT to represent ownership of the position.
pub fn open_position_handler(
    ctx: Context<OpenPosition>,
    tick_lower_index: i32,
    tick_upper_index: i32,
    liquidity: u128,
) -> Result<()> {
    ctx.accounts.validate(tick_lower_index, tick_upper_index)?;

    // Calculate token amounts needed
    let sqrt_price_lower = sqrt_price_from_tick_index(tick_lower_index)?;
    let sqrt_price_upper = sqrt_price_from_tick_index(tick_upper_index)?;
    let (amount_x, amount_y) = get_amounts_for_liquidity(
        ctx.accounts.pool.sqrt_price,
        sqrt_price_lower,
        sqrt_price_upper,
        liquidity,
        true,
    )?;

    // Update ticks
    ctx.accounts.update_tick(&ctx.accounts.tick_array_lower, tick_lower_index, liquidity, false)?;
    ctx.accounts.update_tick(&ctx.accounts.tick_array_upper, tick_upper_index, liquidity, true)?;

    // Get fee growth checkpoints
    let (fee_growth_inside_x, fee_growth_inside_y) = 
        ctx.accounts.get_fee_growth_inside(tick_lower_index, tick_upper_index)?;

    // Update pool liquidity if position is in range
    let pool = &mut ctx.accounts.pool;
    if pool.is_price_in_range(tick_lower_index, tick_upper_index) {
        pool.liquidity = pool.liquidity
            .checked_add(liquidity)
            .ok_or(ClammError::LiquidityOverflow)?;
    }

    // Initialize position
    ctx.accounts.initialize_position(
        tick_lower_index,
        tick_upper_index,
        liquidity,
        fee_growth_inside_x,
        fee_growth_inside_y,
    );

    // Mint position NFT to owner
    ctx.accounts.mint_position_nft()?;

    // Deposit tokens
    ctx.accounts.deposit_tokens(amount_x, amount_y)?;

    Ok(())
}
