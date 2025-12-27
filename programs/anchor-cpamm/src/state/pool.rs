use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    /// First token mint (sorted: mint_x < mint_y lexicographically)
    pub token_mint_x: Pubkey,
    /// Second token mint
    pub token_mint_y: Pubkey,
    /// Current sqrt(price) in Q64.64 format
    /// sqrt_price = sqrt(amount_y / amount_x)
    pub sqrt_price: u128,
    /// Current tick index derived from sqrt_price
    pub tick_current_index: i32,
    /// Total liquidity active at the current tick
    pub liquidity: u128,
    /// Fee rate in hundredths of a basis point (1e-6)
    /// e.g., 3000 = 0.30%
    pub fee_rate: u16,
    /// Tick spacing for this pool
    pub tick_spacing: u16,
    /// Cumulative fees per unit of liquidity for token X (Q64.64)
    pub fee_growth_global_x: u128,
    /// Cumulative fees per unit of liquidity for token Y (Q64.64)
    pub fee_growth_global_y: u128,
    /// PDA bump seed
    pub bump: u8,
}

impl Pool {
    pub const LEN: usize = 8 + // discriminator
        32 + // token_mint_x
        32 + // token_mint_y
        16 + // sqrt_price
        4 +  // tick_current_index
        16 + // liquidity
        2 +  // fee_rate
        2 +  // tick_spacing
        16 + // fee_growth_global_x
        16 + // fee_growth_global_y
        1;   // bump

    pub fn is_price_in_range(&self, tick_lower: i32, tick_upper: i32) -> bool {
        self.tick_current_index >= tick_lower && self.tick_current_index < tick_upper
    }
}
