use anchor_lang::prelude::*;
use crate::state::{Pool, TickArray};
use crate::errors::ClammError;
use crate::constants::TICK_ARRAY_SIZE;

#[derive(Accounts)]
#[instruction(start_tick_index: i32)]
pub struct InitializeTickArray<'info> {
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = payer,
        space = TickArray::LEN,
        seeds = [b"tick_array", pool.key().as_ref(), &start_tick_index.to_le_bytes()],
        bump
    )]
    pub tick_array: AccountLoader<'info, TickArray>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeTickArray<'info> {
    pub fn validate(&self, start_tick_index: i32) -> Result<()> {
        let tick_spacing = self.pool.tick_spacing as i32;
        let ticks_in_array = (TICK_ARRAY_SIZE as i32) * tick_spacing;

        require!(
            start_tick_index % ticks_in_array == 0,
            ClammError::InvalidTickArrayStart
        );

        Ok(())
    }

    pub fn initialize(&self, start_tick_index: i32) -> Result<()> {
        let mut tick_array = self.tick_array.load_init()?;
        tick_array.pool = self.pool.key();
        tick_array.start_tick_index = start_tick_index;
        Ok(())
    }
}

/// Initialize a TickArray for a price range. Arrays are created lazily
/// and must be aligned to TICK_ARRAY_SIZE * tick_spacing.
pub fn initialize_tick_array_handler(ctx: Context<InitializeTickArray>, start_tick_index: i32) -> Result<()> {
    ctx.accounts.validate(start_tick_index)?;
    ctx.accounts.initialize(start_tick_index)
}
