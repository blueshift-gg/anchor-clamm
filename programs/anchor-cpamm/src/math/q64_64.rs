//! Q64.64 fixed-point arithmetic for sqrt_price calculations.
//!
//! Q64.64: 64 bits integer, 64 bits fraction, stored in u128.
//! Used only for sqrt_price - liquidity and token amounts are plain integers.

use crate::errors::ClammError;
use anchor_lang::prelude::*;

pub const Q64: u128 = 1u128 << 64;
pub const Q64_RESOLUTION: u8 = 64;

// Multiply two u128 and shift right by 64, with rounding option
pub fn mul_shift_64(a: u128, b: u128, round_up: bool) -> u128 {
    let (hi, lo) = full_mul_u128(a, b);
    let result = (hi << 64) | (lo >> 64);
    if round_up && (lo & ((1u128 << 64) - 1)) > 0 {
        result + 1
    } else {
        result
    }
}

// Shift left by 64 then divide
pub fn shift_64_div(a: u128, b: u128, round_up: bool) -> Result<u128> {
    require!(b > 0, ClammError::MathOverflow);
    let (hi, lo) = (a >> 64, a << 64);
    let result = div_u256_by_u128(hi, lo, b);
    if round_up && mul_shift_64(result, b, false) < a << 64 {
        Ok(result + 1)
    } else {
        Ok(result)
    }
}

// (a * b) / c with full precision
pub fn mul_div(a: u128, b: u128, c: u128, round_up: bool) -> Result<u128> {
    require!(c > 0, ClammError::MathOverflow);
    let (hi, lo) = full_mul_u128(a, b);
    let result = div_u256_by_u128(hi, lo, c);
    if round_up {
        let (check_hi, check_lo) = full_mul_u128(result, c);
        if check_hi < hi || (check_hi == hi && check_lo < lo) {
            return Ok(result + 1);
        }
    }
    Ok(result)
}

// Full u128 * u128 -> (high, low) parts
fn full_mul_u128(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a & 0xFFFFFFFFFFFFFFFF;
    let a_hi = a >> 64;
    let b_lo = b & 0xFFFFFFFFFFFFFFFF;
    let b_hi = b >> 64;

    let p0 = a_lo * b_lo;
    let p1 = a_lo * b_hi;
    let p2 = a_hi * b_lo;
    let p3 = a_hi * b_hi;

    let (lo, carry0) = p0.overflowing_add(p1 << 64);
    let (lo, carry1) = lo.overflowing_add(p2 << 64);
    let hi = p3 + (p1 >> 64) + (p2 >> 64) + carry0 as u128 + carry1 as u128;

    (hi, lo)
}

// Divide 256-bit number (hi, lo) by u128 divisor
fn div_u256_by_u128(hi: u128, lo: u128, divisor: u128) -> u128 {
    if hi == 0 {
        return lo / divisor;
    }

    // Binary long division
    let mut quotient: u128 = 0;
    let mut remainder: u128 = 0;

    for i in (0..256).rev() {
        remainder <<= 1;
        let bit = if i >= 128 {
            (hi >> (i - 128)) & 1
        } else {
            (lo >> i) & 1
        };
        remainder |= bit;

        if remainder >= divisor {
            remainder -= divisor;
            if i < 128 {
                quotient |= 1 << i;
            }
        }
    }

    quotient
}

/// Integer square root using Newton's method (binary search).
/// Returns floor(sqrt(n)).
pub fn sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    // Initial guess: 2^((log2(n) + 1) / 2)
    let mut x = 1u128 << ((128 - n.leading_zeros() + 1) / 2);

    // Newton's method: x_{n+1} = (x_n + n / x_n) / 2
    loop {
        let x1 = (x + n / x) >> 1;
        if x1 >= x {
            return x;
        }
        x = x1;
    }
}

/// Compute sqrt_price in Q64.64 from token amounts.
/// sqrt_price = sqrt(amount_y / amount_x) * 2^64
/// 
/// We compute this as: sqrt(amount_y) * 2^64 / sqrt(amount_x)
/// This avoids overflow issues with large intermediate values.
pub fn sqrt_price_from_amounts(amount_x: u64, amount_y: u64) -> Result<u128> {
    require!(amount_x > 0 && amount_y > 0, ClammError::InvalidTokenAmounts);

    // sqrt_price = sqrt(y/x) * 2^64
    //            = (sqrt(y) * 2^64) / sqrt(x)
    //            = (sqrt(y) << 64) / sqrt(x)
    let sqrt_y = sqrt_u128(amount_y as u128);
    let sqrt_x = sqrt_u128(amount_x as u128);

    // (sqrt_y << 64) / sqrt_x
    // Use 256-bit division: (sqrt_y << 64) as 256-bit is (0, sqrt_y << 64)
    let numerator = (sqrt_y as u128) << 64;
    let result = numerator / sqrt_x;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_shift_64() {
        let one = Q64;
        // 1.0 * 1.0 = 1.0
        assert_eq!(mul_shift_64(one, one, false), one);
        // 2.0 * 2.0 = 4.0
        assert_eq!(mul_shift_64(2 * Q64, 2 * Q64, false), 4 * Q64);
    }

    #[test]
    fn test_mul_div() {
        // (100 * 50) / 25 = 200
        assert_eq!(mul_div(100, 50, 25, false).unwrap(), 200);
    }

    #[test]
    fn test_sqrt_u128() {
        assert_eq!(sqrt_u128(0), 0);
        assert_eq!(sqrt_u128(1), 1);
        assert_eq!(sqrt_u128(4), 2);
        assert_eq!(sqrt_u128(9), 3);
        assert_eq!(sqrt_u128(100), 10);
        assert_eq!(sqrt_u128(10000), 100);
        // Test large number
        assert_eq!(sqrt_u128(1u128 << 64), 1u128 << 32);
    }

    #[test]
    fn test_sqrt_price_from_amounts() {
        // 1:1 ratio -> sqrt_price = 1.0 in Q64.64 = 2^64
        let sqrt_price = sqrt_price_from_amounts(1_000_000, 1_000_000).unwrap();
        let diff = if sqrt_price > Q64 { sqrt_price - Q64 } else { Q64 - sqrt_price };
        assert!(diff < Q64 / 1000, "1:1 ratio should give sqrt_price ~= 2^64");

        // 1:100 ratio -> price = 100, sqrt_price = 10 in Q64.64
        let sqrt_price = sqrt_price_from_amounts(1_000_000, 100_000_000).unwrap();
        let expected = 10 * Q64;
        let diff = if sqrt_price > expected { sqrt_price - expected } else { expected - sqrt_price };
        assert!(diff < Q64 / 100, "1:100 ratio should give sqrt_price ~= 10 * 2^64");
    }
}
