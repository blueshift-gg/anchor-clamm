use anchor_lang::prelude::*;

/// Position is represented as an NFT. Whoever holds the position_mint token owns
/// the position and can collect fees / modify liquidity.
#[account]
pub struct Position {
    /// The pool this position belongs to
    pub pool: Pubkey,
    /// NFT mint representing ownership of this position
    pub position_mint: Pubkey,
    /// Lower tick bound (inclusive)
    pub tick_lower_index: i32,
    /// Upper tick bound (exclusive)
    pub tick_upper_index: i32,
    /// Liquidity owned by this position
    pub liquidity: u128,
    /// Fee growth inside at last position update for token X (checkpoint)
    pub fee_growth_inside_last_x: u128,
    /// Fee growth inside at last position update for token Y
    pub fee_growth_inside_last_y: u128,
    /// Fees owed to position owner for token X
    pub tokens_owed_x: u64,
    /// Fees owed to position owner for token Y
    pub tokens_owed_y: u64,
}

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        32 + // position_mint
        4 +  // tick_lower_index
        4 +  // tick_upper_index
        16 + // liquidity
        16 + // fee_growth_inside_last_x
        16 + // fee_growth_inside_last_y
        8 +  // tokens_owed_x
        8;   // tokens_owed_y

    /// Update fees owed based on fee growth since last checkpoint
    pub fn update_fees_owed(
        &mut self,
        fee_growth_inside_x: u128,
        fee_growth_inside_y: u128,
    ) {
        // Calculate fees earned since last update using wrapping subtraction
        // This handles overflow correctly since fee_growth only increases
        let fee_growth_delta_x = fee_growth_inside_x.wrapping_sub(self.fee_growth_inside_last_x);
        let fee_growth_delta_y = fee_growth_inside_y.wrapping_sub(self.fee_growth_inside_last_y);

        // fees = (fee_growth_delta * liquidity) >> 64
        // fee_growth is Q64.64, liquidity is plain u128
        let fees_x = ((fee_growth_delta_x as u128) * self.liquidity) >> 64;
        let fees_y = ((fee_growth_delta_y as u128) * self.liquidity) >> 64;

        self.tokens_owed_x = self.tokens_owed_x.saturating_add(fees_x as u64);
        self.tokens_owed_y = self.tokens_owed_y.saturating_add(fees_y as u64);

        // Update checkpoints
        self.fee_growth_inside_last_x = fee_growth_inside_x;
        self.fee_growth_inside_last_y = fee_growth_inside_y;
    }
}
