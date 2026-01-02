use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod anchor_clamm {
    use super::*;

    /// Initialize a new pool with two tokens.
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        amount_x: u64,
        amount_y: u64,
        fee_rate: u16,
        tick_spacing: u16,
    ) -> Result<()> {
        instructions::initialize_pool::initialize_pool_handler(ctx, amount_x, amount_y, fee_rate, tick_spacing)
    }

    /// Initialize a tick array for a price range.
    pub fn initialize_tick_array(
        ctx: Context<InitializeTickArray>,
        start_tick_index: i32,
    ) -> Result<()> {
        instructions::initialize_tick_array::initialize_tick_array_handler(ctx, start_tick_index)
    }

    /// Open a new liquidity position.
    pub fn open_position(
        ctx: Context<OpenPosition>,
        tick_lower_index: i32,
        tick_upper_index: i32,
        liquidity: u128,
    ) -> Result<()> {
        instructions::open_position::open_position_handler(ctx, tick_lower_index, tick_upper_index, liquidity)
    }

    /// Close a position and reclaim rent.
    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        instructions::close_position::close_position_handler(ctx)
    }

    /// Add liquidity to an existing position.
    pub fn add_liquidity(ctx: Context<ModifyLiquidity>, liquidity_delta: u128) -> Result<()> {
        instructions::modify_liquidity::add_liquidity_handler(ctx, liquidity_delta)
    }

    /// Remove liquidity from a position.
    pub fn remove_liquidity(ctx: Context<ModifyLiquidity>, liquidity_delta: u128) -> Result<()> {
        instructions::modify_liquidity::remove_liquidity_handler(ctx, liquidity_delta)
    }

    /// Collect accumulated fees from a position.
    pub fn collect_fees(ctx: Context<CollectFees>) -> Result<()> {
        instructions::collect_fees::collect_fees_handler(ctx)
    }

    /// Execute a swap.
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        zero_for_one: bool,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::swap_handler(ctx, amount_in, zero_for_one, min_amount_out)
    }
}
