use anchor_lang::prelude::*;

/// Tick represents a price boundary where liquidity changes.
/// Uses zero_copy for efficient access within TickArray.
#[zero_copy(unsafe)]
#[repr(C)]
#[derive(Default)]
pub struct Tick {
    /// Whether this tick has any liquidity referencing it (1 = true, 0 = false)
    pub initialized: u8,
    /// Padding for alignment
    pub _padding: [u8; 7],
    /// Net liquidity change when crossing this tick left-to-right
    /// Stored as two i64s because i128 isn't Pod-safe
    pub liquidity_net_lo: i64,
    pub liquidity_net_hi: i64,
    /// Total liquidity referencing this tick as either bound
    pub liquidity_gross: u128,
    /// Fee growth on the "outside" of this tick for token X
    pub fee_growth_outside_x: u128,
    /// Fee growth on the "outside" of this tick for token Y
    pub fee_growth_outside_y: u128,
}

impl Tick {
    pub const LEN: usize = 8 + 8 + 8 + 16 + 16 + 16; // 72 bytes

    /// Get liquidity_net as i128
    pub fn get_liquidity_net(&self) -> i128 {
        ((self.liquidity_net_hi as i128) << 64) | (self.liquidity_net_lo as u64 as i128)
    }

    /// Set liquidity_net from i128
    pub fn set_liquidity_net(&mut self, value: i128) {
        self.liquidity_net_lo = value as i64;
        self.liquidity_net_hi = (value >> 64) as i64;
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized != 0
    }

    /// Initialize tick with fee growth based on current tick position
    pub fn initialize(
        &mut self,
        tick_index: i32,
        tick_current: i32,
        fee_growth_global_x: u128,
        fee_growth_global_y: u128,
    ) {
        self.initialized = 1;
        if tick_index <= tick_current {
            self.fee_growth_outside_x = fee_growth_global_x;
            self.fee_growth_outside_y = fee_growth_global_y;
        } else {
            self.fee_growth_outside_x = 0;
            self.fee_growth_outside_y = 0;
        }
    }

    /// Update liquidity when a position uses this tick as a bound
    pub fn update(&mut self, liquidity_delta: i128, is_upper_tick: bool) {
        self.liquidity_gross = if liquidity_delta >= 0 {
            self.liquidity_gross + liquidity_delta as u128
        } else {
            self.liquidity_gross - (-liquidity_delta) as u128
        };

        let current_net = self.get_liquidity_net();
        let new_net = if is_upper_tick {
            current_net - liquidity_delta
        } else {
            current_net + liquidity_delta
        };
        self.set_liquidity_net(new_net);

        if self.liquidity_gross == 0 {
            self.initialized = 0;
            self.set_liquidity_net(0);
        }
    }

    /// Flip fee growth when price crosses this tick
    pub fn cross(&mut self, fee_growth_global_x: u128, fee_growth_global_y: u128) {
        self.fee_growth_outside_x = fee_growth_global_x.wrapping_sub(self.fee_growth_outside_x);
        self.fee_growth_outside_y = fee_growth_global_y.wrapping_sub(self.fee_growth_outside_y);
    }
}
