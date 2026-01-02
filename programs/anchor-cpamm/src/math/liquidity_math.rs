//! Liquidity math: calculate token amounts for positions

use crate::errors::ClammError;
use anchor_lang::prelude::*;

/// Calculate liquidity from token amounts for a given position range.
///
/// For a position [tick_lower, tick_upper] with current price:
/// - If price <= lower: position is 100% token X
/// - If price >= upper: position is 100% token Y  
/// - If lower < price < upper: position has both tokens
pub fn get_liquidity_for_amounts(
    sqrt_price_current: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    amount_x: u64,
    amount_y: u64,
) -> Result<u128> {
    if sqrt_price_current <= sqrt_price_lower {
        // Price below range: position is entirely X
        get_liquidity_for_amount_x(sqrt_price_lower, sqrt_price_upper, amount_x)
    } else if sqrt_price_current >= sqrt_price_upper {
        // Price above range: position is entirely Y
        get_liquidity_for_amount_y(sqrt_price_lower, sqrt_price_upper, amount_y)
    } else {
        // Price in range: use minimum of both calculations
        let liquidity_x = get_liquidity_for_amount_x(sqrt_price_current, sqrt_price_upper, amount_x)?;
        let liquidity_y = get_liquidity_for_amount_y(sqrt_price_lower, sqrt_price_current, amount_y)?;
        Ok(liquidity_x.min(liquidity_y))
    }
}

/// Liquidity from amount X: L = Δx * √P_upper * √P_lower / (√P_upper - √P_lower)
fn get_liquidity_for_amount_x(
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    amount: u64,
) -> Result<u128> {
    let sqrt_price_diff = sqrt_price_upper - sqrt_price_lower;
    if sqrt_price_diff == 0 {
        return Ok(0);
    }
    
    // L = amount * sqrt_price_lower * sqrt_price_upper / (sqrt_price_upper - sqrt_price_lower)
    // All values in Q64.64, so we need to handle precision carefully
    
    // (amount << 64) to convert to Q64.64
    let amount_q64 = (amount as u128) << 64;
    
    // Numerator: amount * sqrt_price_lower * sqrt_price_upper / 2^64 (to keep in Q64.64)
    let numerator = crate::math::q64_64::mul_div(
        amount_q64,
        crate::math::q64_64::mul_shift_64(sqrt_price_lower, sqrt_price_upper, false),
        1u128 << 64,
        false,
    )?;
    
    // Divide by price diff
    Ok(numerator / sqrt_price_diff)
}

/// Liquidity from amount Y: L = Δy / (√P_upper - √P_lower)
fn get_liquidity_for_amount_y(
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    amount: u64,
) -> Result<u128> {
    let sqrt_price_diff = sqrt_price_upper - sqrt_price_lower;
    if sqrt_price_diff == 0 {
        return Ok(0);
    }
    
    // L = (amount << 64) / sqrt_price_diff
    let amount_q64 = (amount as u128) << 64;
    Ok(amount_q64 / sqrt_price_diff)
}

/// Calculate token amounts required for a given liquidity in a position.
pub fn get_amounts_for_liquidity(
    sqrt_price_current: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<(u64, u64)> {
    let amount_x = if sqrt_price_current >= sqrt_price_upper {
        // Price above range: no X needed
        0
    } else {
        let sqrt_price_x = sqrt_price_current.max(sqrt_price_lower);
        get_amount_x_delta(sqrt_price_x, sqrt_price_upper, liquidity, round_up)?
    };

    let amount_y = if sqrt_price_current <= sqrt_price_lower {
        // Price below range: no Y needed
        0
    } else {
        let sqrt_price_y = sqrt_price_current.min(sqrt_price_upper);
        get_amount_y_delta(sqrt_price_lower, sqrt_price_y, liquidity, round_up)?
    };

    Ok((amount_x, amount_y))
}

/// Amount X = L * (√P_upper - √P_lower) / (√P_upper * √P_lower)
/// = L * (1/√P_lower - 1/√P_upper)
pub fn get_amount_x_delta(
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u64> {
    if liquidity == 0 || sqrt_price_lower == sqrt_price_upper {
        return Ok(0);
    }

    let sqrt_price_diff = sqrt_price_upper - sqrt_price_lower;
    
    // amount = L * (sqrt_upper - sqrt_lower) / (sqrt_upper * sqrt_lower) * 2^64
    // = L * sqrt_price_diff / (sqrt_upper * sqrt_lower / 2^64)
    
    let denominator = crate::math::q64_64::mul_shift_64(sqrt_price_upper, sqrt_price_lower, false);
    
    let result = crate::math::q64_64::mul_div(
        liquidity,
        sqrt_price_diff,
        denominator,
        round_up,
    )?;

    // Result is already in integer units (not Q64.64)
    // numerator = L * diff (where diff is Q64.64, so this is L * diff_real * 2^64)
    // denominator = (upper * lower) >> 64 (which is upper_real * lower_real * 2^64)
    // result = (L * diff_real * 2^64) / (upper_real * lower_real * 2^64) = L * diff_real / (upper_real * lower_real)
    if result > u64::MAX as u128 {
        return Err(ClammError::TokenAmountOverflow.into());
    }
    
    Ok(result as u64)
}

/// Amount Y = L * (√P_upper - √P_lower)
pub fn get_amount_y_delta(
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u64> {
    if liquidity == 0 || sqrt_price_lower == sqrt_price_upper {
        return Ok(0);
    }

    let sqrt_price_diff = sqrt_price_upper - sqrt_price_lower;
    
    // amount = L * sqrt_price_diff / 2^64
    let result = crate::math::q64_64::mul_shift_64(liquidity, sqrt_price_diff, round_up);
    
    if result > u64::MAX as u128 {
        return Err(ClammError::TokenAmountOverflow.into());
    }
    
    Ok(result as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_amount_y_delta() {
        // Use smaller values to avoid overflow
        // sqrt_price in Q64.64: 1.0 = 2^64, 1.5 = 1.5 * 2^64
        let sqrt_price_lower = 1u128 << 64; // 1.0
        let sqrt_price_upper = (3u128 << 64) / 2; // 1.5
        let liquidity = 1000u128; // small liquidity
        
        let amount = get_amount_y_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false).unwrap();
        // Expected: L * (1.5 - 1.0) = 1000 * 0.5 = 500
        assert_eq!(amount, 500);
    }
}
