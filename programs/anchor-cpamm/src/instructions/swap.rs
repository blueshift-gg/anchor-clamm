use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Pool, TickArray};
use crate::errors::ClammError;
use crate::math::{
    tick_math::sqrt_price_from_tick_index,
    sqrt_price_math::compute_swap_step,
};
use crate::constants::TICK_ARRAY_SIZE;

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub pool: Account<'info, Pool>,

    /// Current or first tick array in swap direction
    #[account(
        mut,
        constraint = tick_array_0.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_0: AccountLoader<'info, TickArray>,

    /// Optional: next tick array if swap crosses boundary
    #[account(
        mut,
        constraint = tick_array_1.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_1: Option<AccountLoader<'info, TickArray>>,

    /// Optional: another tick array for large swaps
    #[account(
        mut,
        constraint = tick_array_2.load()?.pool == pool.key() @ ClammError::InvalidTick
    )]
    pub tick_array_2: Option<AccountLoader<'info, TickArray>>,

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

    #[account(mut)]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_out: Account<'info, TokenAccount>,

    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    pub fn validate(&self, amount_in: u64) -> Result<()> {
        require!(amount_in > 0, ClammError::ZeroLiquidity);
        require!(self.pool.liquidity > 0, ClammError::ZeroLiquidity);
        Ok(())
    }

    pub fn transfer_in(&self, amount: u64, zero_for_one: bool) -> Result<()> {
        let (from, to) = if zero_for_one {
            (self.user_token_in.to_account_info(), self.token_vault_x.to_account_info())
        } else {
            (self.user_token_in.to_account_info(), self.token_vault_y.to_account_info())
        };

        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer { from, to, authority: self.user.to_account_info() },
            ),
            amount,
        )
    }

    pub fn transfer_out(&self, amount: u64, zero_for_one: bool) -> Result<()> {
        let seeds = &[
            b"pool".as_ref(),
            self.pool.token_mint_x.as_ref(),
            self.pool.token_mint_y.as_ref(),
            &[self.pool.bump],
        ];
        let signer = &[&seeds[..]];

        let (from, to) = if zero_for_one {
            (self.token_vault_y.to_account_info(), self.user_token_out.to_account_info())
        } else {
            (self.token_vault_x.to_account_info(), self.user_token_out.to_account_info())
        };

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer { from, to, authority: self.pool.to_account_info() },
                signer,
            ),
            amount,
        )
    }
}

/// Execute a swap. If zero_for_one is true, swaps token X for Y (price decreases).
pub fn swap_handler(
    ctx: Context<Swap>,
    amount_in: u64,
    zero_for_one: bool,
    min_amount_out: u64,
) -> Result<()> {
    ctx.accounts.validate(amount_in)?;

    let pool = &mut ctx.accounts.pool;
    let tick_spacing = pool.tick_spacing;
    let fee_rate = pool.fee_rate;

    let mut sqrt_price_current = pool.sqrt_price;
    let mut tick_current = pool.tick_current_index;
    let mut liquidity = pool.liquidity;
    let mut amount_remaining = amount_in;
    let mut amount_out_total: u64 = 0;
    let mut fee_growth_global = if zero_for_one {
        pool.fee_growth_global_x
    } else {
        pool.fee_growth_global_y
    };

    let mut iterations = 0;
    const MAX_ITERATIONS: u32 = 100;

    while amount_remaining > 0 && iterations < MAX_ITERATIONS {
        iterations += 1;

        let (next_tick, next_tick_array_index) = find_next_initialized_tick(
            &ctx.accounts.tick_array_0,
            &ctx.accounts.tick_array_1,
            &ctx.accounts.tick_array_2,
            tick_current,
            tick_spacing,
            zero_for_one,
        )?;

        let sqrt_price_target = sqrt_price_from_tick_index(next_tick)?;

        let step_result = compute_swap_step(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_remaining,
            fee_rate,
        )?;

        sqrt_price_current = step_result.sqrt_price_next;
        amount_remaining = amount_remaining.saturating_sub(step_result.amount_in + step_result.fee_amount);
        amount_out_total = amount_out_total.saturating_add(step_result.amount_out);

        if liquidity > 0 && step_result.fee_amount > 0 {
            let fee_growth_delta = ((step_result.fee_amount as u128) << 64) / liquidity;
            fee_growth_global = fee_growth_global.wrapping_add(fee_growth_delta);
        }

        if sqrt_price_current == sqrt_price_target {
            let tick_array = get_tick_array_by_index(
                &ctx.accounts.tick_array_0,
                &ctx.accounts.tick_array_1,
                &ctx.accounts.tick_array_2,
                next_tick_array_index,
            )?;

            let mut tick_array_data = tick_array.load_mut()?;
            if let Some(tick) = tick_array_data.get_tick_mut(next_tick, tick_spacing) {
                if tick.is_initialized() {
                    tick.cross(pool.fee_growth_global_x, pool.fee_growth_global_y);

                    let tick_liquidity_net = tick.get_liquidity_net();
                    let liquidity_net = if zero_for_one {
                        -tick_liquidity_net
                    } else {
                        tick_liquidity_net
                    };

                    if liquidity_net > 0 {
                        liquidity = liquidity.checked_add(liquidity_net as u128)
                            .ok_or(ClammError::LiquidityOverflow)?;
                    } else {
                        liquidity = liquidity.checked_sub((-liquidity_net) as u128)
                            .ok_or(ClammError::LiquidityUnderflow)?;
                    }
                }
            }

            tick_current = if zero_for_one { next_tick - 1 } else { next_tick };
        } else {
            tick_current = crate::math::tick_math::tick_index_from_sqrt_price(sqrt_price_current)?;
        }
    }

    require!(amount_out_total >= min_amount_out, ClammError::SlippageExceeded);

    // Update pool state
    let pool = &mut ctx.accounts.pool;
    pool.sqrt_price = sqrt_price_current;
    pool.tick_current_index = tick_current;
    pool.liquidity = liquidity;
    if zero_for_one {
        pool.fee_growth_global_x = fee_growth_global;
    } else {
        pool.fee_growth_global_y = fee_growth_global;
    }

    // Transfer tokens
    let amount_in_used = amount_in - amount_remaining;
    ctx.accounts.transfer_in(amount_in_used, zero_for_one)?;
    ctx.accounts.transfer_out(amount_out_total, zero_for_one)?;

    Ok(())
}

/// Find the next initialized tick in the swap direction
fn find_next_initialized_tick<'info>(
    tick_array_0: &AccountLoader<'info, TickArray>,
    tick_array_1: &Option<AccountLoader<'info, TickArray>>,
    tick_array_2: &Option<AccountLoader<'info, TickArray>>,
    current_tick: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Result<(i32, usize)> {
    let spacing = tick_spacing as i32;

    // Search in tick_array_0 first
    let ta0 = tick_array_0.load()?;
    if let Some((tick, _)) = search_tick_in_array(&ta0, current_tick, spacing, zero_for_one) {
        return Ok((tick, 0));
    }

    // Search in tick_array_1
    if let Some(ta1_loader) = tick_array_1 {
        let ta1 = ta1_loader.load()?;
        if let Some((tick, _)) = search_tick_in_array(&ta1, current_tick, spacing, zero_for_one) {
            return Ok((tick, 1));
        }
    }

    // Search in tick_array_2
    if let Some(ta2_loader) = tick_array_2 {
        let ta2 = ta2_loader.load()?;
        if let Some((tick, _)) = search_tick_in_array(&ta2, current_tick, spacing, zero_for_one) {
            return Ok((tick, 2));
        }
    }

    // No initialized tick found, return the boundary of the last array
    let ta0 = tick_array_0.load()?;
    let boundary = if zero_for_one {
        ta0.start_tick_index
    } else {
        ta0.start_tick_index + (TICK_ARRAY_SIZE as i32) * spacing - spacing
    };
    Ok((boundary, 0))
}

fn search_tick_in_array(
    tick_array: &TickArray,
    current_tick: i32,
    tick_spacing: i32,
    zero_for_one: bool,
) -> Option<(i32, usize)> {
    let start = tick_array.start_tick_index;
    let end = start + (TICK_ARRAY_SIZE as i32) * tick_spacing;

    if zero_for_one {
        // Search left (lower ticks)
        let mut search_tick = (current_tick / tick_spacing) * tick_spacing;
        if search_tick > current_tick {
            search_tick -= tick_spacing;
        }

        while search_tick >= start {
            if search_tick < end {
                let offset = ((search_tick - start) / tick_spacing) as usize;
                if offset < TICK_ARRAY_SIZE && tick_array.ticks[offset].is_initialized() {
                    return Some((search_tick, offset));
                }
            }
            search_tick -= tick_spacing;
        }
    } else {
        // Search right (higher ticks)
        let mut search_tick = ((current_tick / tick_spacing) + 1) * tick_spacing;

        while search_tick < end {
            if search_tick >= start {
                let offset = ((search_tick - start) / tick_spacing) as usize;
                if offset < TICK_ARRAY_SIZE && tick_array.ticks[offset].is_initialized() {
                    return Some((search_tick, offset));
                }
            }
            search_tick += tick_spacing;
        }
    }

    None
}

fn get_tick_array_by_index<'a, 'info>(
    tick_array_0: &'a AccountLoader<'info, TickArray>,
    tick_array_1: &'a Option<AccountLoader<'info, TickArray>>,
    tick_array_2: &'a Option<AccountLoader<'info, TickArray>>,
    index: usize,
) -> Result<&'a AccountLoader<'info, TickArray>> {
    match index {
        0 => Ok(tick_array_0),
        1 => tick_array_1.as_ref().ok_or(ClammError::TickNotInArray.into()),
        2 => tick_array_2.as_ref().ok_or(ClammError::TickNotInArray.into()),
        _ => Err(ClammError::TickNotInArray.into()),
    }
}
