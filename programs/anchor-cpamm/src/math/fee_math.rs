//! Fee math: calculate fee growth inside position ranges

use crate::state::Tick;

/// Calculate fee growth inside a position's tick range.
///
/// The key formula: fee_growth_inside = global - below - above
///
/// Where:
/// - below = fees accumulated below tick_lower
/// - above = fees accumulated above tick_upper
pub fn get_fee_growth_inside(
    tick_lower: &Tick,
    tick_upper: &Tick,
    tick_lower_index: i32,
    tick_upper_index: i32,
    tick_current_index: i32,
    fee_growth_global_x: u128,
    fee_growth_global_y: u128,
) -> (u128, u128) {
    // Fee growth below the position
    let (fee_growth_below_x, fee_growth_below_y) = if tick_current_index >= tick_lower_index {
        // Current price is at or above tick_lower
        // "outside" of tick_lower means below tick_lower
        (tick_lower.fee_growth_outside_x, tick_lower.fee_growth_outside_y)
    } else {
        // Current price is below tick_lower
        // "outside" of tick_lower means above tick_lower, so we invert
        (
            fee_growth_global_x.wrapping_sub(tick_lower.fee_growth_outside_x),
            fee_growth_global_y.wrapping_sub(tick_lower.fee_growth_outside_y),
        )
    };

    // Fee growth above the position
    let (fee_growth_above_x, fee_growth_above_y) = if tick_current_index < tick_upper_index {
        // Current price is below tick_upper
        // "outside" of tick_upper means above tick_upper
        (tick_upper.fee_growth_outside_x, tick_upper.fee_growth_outside_y)
    } else {
        // Current price is at or above tick_upper
        // "outside" of tick_upper means below tick_upper, so we invert
        (
            fee_growth_global_x.wrapping_sub(tick_upper.fee_growth_outside_x),
            fee_growth_global_y.wrapping_sub(tick_upper.fee_growth_outside_y),
        )
    };

    // Fee growth inside = global - below - above
    // Using wrapping subtraction to handle overflow correctly
    let fee_growth_inside_x = fee_growth_global_x
        .wrapping_sub(fee_growth_below_x)
        .wrapping_sub(fee_growth_above_x);
    let fee_growth_inside_y = fee_growth_global_y
        .wrapping_sub(fee_growth_below_y)
        .wrapping_sub(fee_growth_above_y);

    (fee_growth_inside_x, fee_growth_inside_y)
}

/// Calculate fee from input amount
pub fn calculate_fee(amount: u64, fee_rate: u16) -> u64 {
    // fee_rate is in hundredths of a basis point (1e-6)
    // fee = amount * fee_rate / 1_000_000
    ((amount as u128) * (fee_rate as u128) / 1_000_000) as u64
}

/// Calculate amount after fee
pub fn get_amount_after_fee(amount: u64, fee_rate: u16) -> u64 {
    amount - calculate_fee(amount, fee_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fee() {
        // 0.30% fee on 1,000,000 tokens = 3,000 tokens
        let fee = calculate_fee(1_000_000, 3000);
        assert_eq!(fee, 3000);
    }

    #[test]
    fn test_fee_growth_inside_simple() {
        let mut tick_lower = Tick::default();
        let mut tick_upper = Tick::default();
        
        // Initialize ticks with some fee growth
        tick_lower.fee_growth_outside_x = 100;
        tick_lower.fee_growth_outside_y = 50;
        tick_upper.fee_growth_outside_x = 20;
        tick_upper.fee_growth_outside_y = 10;

        // Global fee growth
        let global_x = 200;
        let global_y = 100;

        // Current tick is in the middle of the range
        let (inside_x, inside_y) = get_fee_growth_inside(
            &tick_lower,
            &tick_upper,
            -100,  // tick_lower_index
            100,   // tick_upper_index
            0,     // tick_current (in range)
            global_x,
            global_y,
        );

        // inside = global - below - above = 200 - 100 - 20 = 80
        assert_eq!(inside_x, 80);
        // inside = global - below - above = 100 - 50 - 10 = 40
        assert_eq!(inside_y, 40);
    }
}
