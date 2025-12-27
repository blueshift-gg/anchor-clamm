//! Sqrt price math: compute price changes during swaps

use crate::errors::ClammError;
use crate::math::q64_64;
use crate::constants::{MIN_SQRT_PRICE, MAX_SQRT_PRICE};
use anchor_lang::prelude::*;

/// Calculate next sqrt price when swapping token X for Y (price decreases).
/// Formula: √P' = L * √P / (L + Δx * √P)
pub fn get_next_sqrt_price_x_up(
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
    add: bool, // true = adding X (price decreases), false = removing X (price increases)
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price);
    }

    // numerator = L * √P
    // denominator = L + amount * √P (if adding) or L - amount * √P (if removing)
    
    let amount_u128 = amount as u128;
    
    // product = amount * sqrt_price / 2^64
    let product = q64_64::mul_shift_64(amount_u128, sqrt_price, false);
    
    // Shift liquidity to match scale: L << 64
    let liquidity_shifted = liquidity << 64;
    
    let denominator = if add {
        liquidity_shifted.checked_add(product)
            .ok_or(ClammError::MathOverflow)?
    } else {
        liquidity_shifted.checked_sub(product)
            .ok_or(ClammError::LiquidityUnderflow)?
    };

    // result = (L << 64) * sqrt_price / denominator
    let result = q64_64::mul_div(liquidity_shifted, sqrt_price, denominator, true)?;
    
    // Clamp to valid range
    if result < MIN_SQRT_PRICE {
        return Err(ClammError::SqrtPriceOutOfBounds.into());
    }
    if result > MAX_SQRT_PRICE {
        return Err(ClammError::SqrtPriceOutOfBounds.into());
    }
    
    Ok(result)
}

/// Calculate next sqrt price when swapping token Y for X (price increases).
/// Formula: √P' = √P + Δy / L
pub fn get_next_sqrt_price_y_down(
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
    add: bool, // true = adding Y (price increases), false = removing Y (price decreases)
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price);
    }

    // delta = (amount << 64) / liquidity
    let amount_shifted = (amount as u128) << 64;
    let delta = amount_shifted / liquidity;

    let result = if add {
        sqrt_price.checked_add(delta)
            .ok_or(ClammError::SqrtPriceOutOfBounds)?
    } else {
        sqrt_price.checked_sub(delta)
            .ok_or(ClammError::SqrtPriceOutOfBounds)?
    };

    if result < MIN_SQRT_PRICE || result > MAX_SQRT_PRICE {
        return Err(ClammError::SqrtPriceOutOfBounds.into());
    }

    Ok(result)
}

/// Compute a single swap step within one tick range.
/// Returns (amount_in, amount_out, sqrt_price_next, fee_amount)
pub fn compute_swap_step(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u16,
) -> Result<SwapStepResult> {
    let zero_for_one = sqrt_price_current >= sqrt_price_target;
    
    // Calculate fee
    let fee_amount = crate::math::fee_math::calculate_fee(amount_remaining, fee_rate);
    let amount_remaining_after_fee = amount_remaining - fee_amount;
    
    // Calculate how much input is needed to reach target price
    let amount_in_to_target = if zero_for_one {
        // Swapping X for Y: calculate X needed to reach target price
        crate::math::liquidity_math::get_amount_x_delta(
            sqrt_price_target,
            sqrt_price_current,
            liquidity,
            true, // round up for input
        )?
    } else {
        // Swapping Y for X: calculate Y needed to reach target price
        crate::math::liquidity_math::get_amount_y_delta(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            true,
        )?
    };
    
    let (sqrt_price_next, amount_in, amount_out) = if amount_remaining_after_fee >= amount_in_to_target {
        // We can reach the target price
        let amount_out = if zero_for_one {
            crate::math::liquidity_math::get_amount_y_delta(
                sqrt_price_target,
                sqrt_price_current,
                liquidity,
                false,
            )?
        } else {
            crate::math::liquidity_math::get_amount_x_delta(
                sqrt_price_current,
                sqrt_price_target,
                liquidity,
                false,
            )?
        };
        (sqrt_price_target, amount_in_to_target, amount_out)
    } else {
        // We can't reach target, calculate how far we can go
        let sqrt_price_next = if zero_for_one {
            get_next_sqrt_price_x_up(sqrt_price_current, liquidity, amount_remaining_after_fee, true)?
        } else {
            get_next_sqrt_price_y_down(sqrt_price_current, liquidity, amount_remaining_after_fee, true)?
        };
        
        let amount_out = if zero_for_one {
            crate::math::liquidity_math::get_amount_y_delta(
                sqrt_price_next,
                sqrt_price_current,
                liquidity,
                false,
            )?
        } else {
            crate::math::liquidity_math::get_amount_x_delta(
                sqrt_price_current,
                sqrt_price_next,
                liquidity,
                false,
            )?
        };
        
        (sqrt_price_next, amount_remaining_after_fee, amount_out)
    };
    
    // Recalculate actual fee based on amount_in used
    let actual_fee = if amount_in == amount_remaining_after_fee {
        fee_amount
    } else {
        crate::math::fee_math::calculate_fee(amount_in, fee_rate)
    };

    Ok(SwapStepResult {
        sqrt_price_next,
        amount_in,
        amount_out,
        fee_amount: actual_fee,
    })
}

#[derive(Debug)]
pub struct SwapStepResult {
    pub sqrt_price_next: u128,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_next_sqrt_price_y_down_add() {
        let sqrt_price = 1u128 << 64; // 1.0
        let liquidity = 1u128 << 64;  // 1.0
        let amount = 1u64;
        
        let next = get_next_sqrt_price_y_down(sqrt_price, liquidity, amount, true).unwrap();
        // Should increase price
        assert!(next > sqrt_price);
    }
}
