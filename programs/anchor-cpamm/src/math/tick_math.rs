//! Tick math: conversions between tick indices and sqrt prices.
//!
//! Uses binary decomposition with precomputed magic constants
//! to efficiently compute sqrt(1.0001)^tick.

use crate::constants::*;
use crate::errors::ClammError;
use anchor_lang::prelude::*;

// Separate constants for positive and negative ticks (like Orca)
// For positive ticks: these are sqrt(1.0001)^(2^i) in Q96.32 format
// For negative ticks: these are 1/sqrt(1.0001)^(2^i) in Q64.64 format

pub fn sqrt_price_from_tick_index(tick: i32) -> Result<u128> {
    require!(tick >= MIN_TICK && tick <= MAX_TICK, ClammError::InvalidTick);

    if tick >= 0 {
        Ok(get_sqrt_price_positive_tick(tick))
    } else {
        Ok(get_sqrt_price_negative_tick(tick))
    }
}

fn get_sqrt_price_positive_tick(tick: i32) -> u128 {
    // Start with 1.0 in Q96.32 format
    let mut ratio: u128 = if tick & 1 != 0 {
        79232123823359799118286999567
    } else {
        79228162514264337593543950336 // 1.0 in Q96.32
    };

    if tick & 2 != 0 {
        ratio = mul_shift_96(ratio, 79236085330515764027303304731);
    }
    if tick & 4 != 0 {
        ratio = mul_shift_96(ratio, 79244008939048815603706035061);
    }
    if tick & 8 != 0 {
        ratio = mul_shift_96(ratio, 79259858533276714757314932305);
    }
    if tick & 16 != 0 {
        ratio = mul_shift_96(ratio, 79291567232598584799939703904);
    }
    if tick & 32 != 0 {
        ratio = mul_shift_96(ratio, 79355022692464371645785046466);
    }
    if tick & 64 != 0 {
        ratio = mul_shift_96(ratio, 79482085999252804386437311141);
    }
    if tick & 128 != 0 {
        ratio = mul_shift_96(ratio, 79736823300114093921829183326);
    }
    if tick & 256 != 0 {
        ratio = mul_shift_96(ratio, 80248749790819932309965073892);
    }
    if tick & 512 != 0 {
        ratio = mul_shift_96(ratio, 81282483887344747381513967011);
    }
    if tick & 1024 != 0 {
        ratio = mul_shift_96(ratio, 83390072131320151908154831281);
    }
    if tick & 2048 != 0 {
        ratio = mul_shift_96(ratio, 87770609709833776024991924138);
    }
    if tick & 4096 != 0 {
        ratio = mul_shift_96(ratio, 97234110755111693312479820773);
    }
    if tick & 8192 != 0 {
        ratio = mul_shift_96(ratio, 119332217159966728226237229890);
    }
    if tick & 16384 != 0 {
        ratio = mul_shift_96(ratio, 179736315981702064433883588727);
    }
    if tick & 32768 != 0 {
        ratio = mul_shift_96(ratio, 407748233172238350107850275304);
    }
    if tick & 65536 != 0 {
        ratio = mul_shift_96(ratio, 2098478828474011932436660412517);
    }
    if tick & 131072 != 0 {
        ratio = mul_shift_96(ratio, 55581415166113811149459800483533);
    }
    if tick & 262144 != 0 {
        ratio = mul_shift_96(ratio, 38992368544603139932233054999993551);
    }

    // Convert from Q96.32 to Q64.64
    ratio >> 32
}

fn get_sqrt_price_negative_tick(tick: i32) -> u128 {
    let abs_tick = tick.abs();

    // Start with 1.0 in Q64.64 format
    let mut ratio: u128 = if abs_tick & 1 != 0 {
        18445821805675392311
    } else {
        18446744073709551616 // 1.0 in Q64.64
    };

    if abs_tick & 2 != 0 {
        ratio = (ratio * 18444899583751176498) >> 64;
    }
    if abs_tick & 4 != 0 {
        ratio = (ratio * 18443055278223354162) >> 64;
    }
    if abs_tick & 8 != 0 {
        ratio = (ratio * 18439367220385604838) >> 64;
    }
    if abs_tick & 16 != 0 {
        ratio = (ratio * 18431993317065449817) >> 64;
    }
    if abs_tick & 32 != 0 {
        ratio = (ratio * 18417254355718160513) >> 64;
    }
    if abs_tick & 64 != 0 {
        ratio = (ratio * 18387811781193591352) >> 64;
    }
    if abs_tick & 128 != 0 {
        ratio = (ratio * 18329067761203520168) >> 64;
    }
    if abs_tick & 256 != 0 {
        ratio = (ratio * 18212142134806087854) >> 64;
    }
    if abs_tick & 512 != 0 {
        ratio = (ratio * 17980523815641551639) >> 64;
    }
    if abs_tick & 1024 != 0 {
        ratio = (ratio * 17526086738831147013) >> 64;
    }
    if abs_tick & 2048 != 0 {
        ratio = (ratio * 16651378430235024244) >> 64;
    }
    if abs_tick & 4096 != 0 {
        ratio = (ratio * 15030750278693429944) >> 64;
    }
    if abs_tick & 8192 != 0 {
        ratio = (ratio * 12247334978882834399) >> 64;
    }
    if abs_tick & 16384 != 0 {
        ratio = (ratio * 8131365268884726200) >> 64;
    }
    if abs_tick & 32768 != 0 {
        ratio = (ratio * 3584323654723342297) >> 64;
    }
    if abs_tick & 65536 != 0 {
        ratio = (ratio * 696457651847595233) >> 64;
    }
    if abs_tick & 131072 != 0 {
        ratio = (ratio * 26294789957452057) >> 64;
    }
    if abs_tick & 262144 != 0 {
        ratio = (ratio * 37481735321082) >> 64;
    }

    ratio
}

// Multiply and shift right by 96 (for Q96.32 math)
fn mul_shift_96(a: u128, b: u128) -> u128 {
    let a_hi = a >> 64;
    let a_lo = a & 0xFFFFFFFFFFFFFFFF;
    let b_hi = b >> 64;
    let b_lo = b & 0xFFFFFFFFFFFFFFFF;

    let p0 = a_lo * b_lo;
    let p1 = a_lo * b_hi;
    let p2 = a_hi * b_lo;
    let p3 = a_hi * b_hi;

    // We want (a * b) >> 96
    // The result is: p3 << 32 + (p1 + p2) >> 32 + p0 >> 96
    let mid = p1 + p2;
    (p3 << 32) + (mid >> 32) + (p0 >> 96)
}

pub fn tick_index_from_sqrt_price(sqrt_price: u128) -> Result<i32> {
    require!(
        sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
        ClammError::InvalidSqrtPrice
    );

    // Get MSB position
    let msb: u32 = 128 - sqrt_price.leading_zeros() - 1;
    let log2p_integer_x32 = ((msb as i128) - 64) << 32;

    // Iterative log2 approximation for fractional part
    let mut bit: i128 = 0x8000_0000_0000_0000i128;
    let mut precision = 0;
    let mut log2p_fraction_x64: i128 = 0;

    let mut r = if msb >= 64 {
        sqrt_price >> (msb - 63)
    } else {
        sqrt_price << (63 - msb)
    };

    while bit > 0 && precision < 14 {
        r = r * r;
        let is_r_more_than_two = r >> 127;
        r >>= 63 + is_r_more_than_two;
        log2p_fraction_x64 += bit * (is_r_more_than_two as i128);
        bit >>= 1;
        precision += 1;
    }

    let log2p_fraction_x32 = log2p_fraction_x64 >> 32;
    let log2p_x32 = log2p_integer_x32 + log2p_fraction_x32;

    // Transform from base 2 to base 1.0001
    // log_1.0001(x) = log2(x) / log2(1.0001)
    const LOG_B_2_X32: i128 = 59543866431248i128;
    let logbp_x64 = log2p_x32 * LOG_B_2_X32;

    // Error margins for precision
    const LOG_B_P_ERR_MARGIN_LOWER_X64: i128 = 184467440737095516i128;
    const LOG_B_P_ERR_MARGIN_UPPER_X64: i128 = 15793534762490258745i128;

    let tick_low: i32 = ((logbp_x64 - LOG_B_P_ERR_MARGIN_LOWER_X64) >> 64) as i32;
    let tick_high: i32 = ((logbp_x64 + LOG_B_P_ERR_MARGIN_UPPER_X64) >> 64) as i32;

    if tick_low == tick_high {
        Ok(tick_low)
    } else {
        let actual_tick_high_sqrt_price = sqrt_price_from_tick_index(tick_high)?;
        if actual_tick_high_sqrt_price <= sqrt_price {
            Ok(tick_high)
        } else {
            Ok(tick_low)
        }
    }
}

pub fn check_tick(tick: i32, tick_spacing: u16) -> Result<()> {
    require!(tick >= MIN_TICK && tick <= MAX_TICK, ClammError::InvalidTick);
    require!(tick % (tick_spacing as i32) == 0, ClammError::InvalidTick);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_zero() {
        let sqrt_price = sqrt_price_from_tick_index(0).unwrap();
        let one = 1u128 << 64;
        assert!((sqrt_price as i128 - one as i128).abs() < 1000);
    }

    #[test]
    fn test_tick_bounds() {
        assert!(sqrt_price_from_tick_index(MIN_TICK).is_ok());
        assert!(sqrt_price_from_tick_index(MAX_TICK).is_ok());
        assert!(sqrt_price_from_tick_index(MIN_TICK - 1).is_err());
        assert!(sqrt_price_from_tick_index(MAX_TICK + 1).is_err());
    }

    #[test]
    fn test_round_trip() {
        for tick in [-10000, -1000, -100, 0, 100, 1000, 10000].iter() {
            let sqrt_price = sqrt_price_from_tick_index(*tick).unwrap();
            let recovered_tick = tick_index_from_sqrt_price(sqrt_price).unwrap();
            assert_eq!(*tick, recovered_tick);
        }
    }

    #[test]
    fn test_price_increasing() {
        let price_neg = sqrt_price_from_tick_index(-100).unwrap();
        let price_zero = sqrt_price_from_tick_index(0).unwrap();
        let price_pos = sqrt_price_from_tick_index(100).unwrap();
        assert!(price_neg < price_zero);
        assert!(price_zero < price_pos);
    }
}
