//! Constants for the Concentrated Liquidity AMM

// Tick bounds - corresponds to price range of ~2^-128 to ~2^128
pub const MIN_TICK: i32 = -443636;
pub const MAX_TICK: i32 = 443636;

// Tick array configuration
pub const TICK_ARRAY_SIZE: usize = 88;
pub const DEFAULT_TICK_SPACING: u16 = 64;

// Fixed-point constants
pub const Q64: u128 = 1u128 << 64;

// Fee configuration (using 1M denominator for 0.0001% granularity)
pub const FEE_RATE_DENOMINATOR: u64 = 1_000_000;
pub const MAX_FEE_RATE: u64 = 100_000; // 10% max

// Price bounds in Q64.64 format
pub const MIN_SQRT_PRICE: u128 = 4295048016;
pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055;

// Magic constants for tick_to_sqrt_price calculation.
// Each is 1/sqrt(1.0001)^(2^i) in Q128.128 format.
// Using reciprocals lets us handle negative ticks by multiplying,
// then inverting at the end only for positive ticks.
pub const MAGIC_SQRT_1: u128 = 0xfffcb933bd6fad37aa2d162d1a594001;
pub const MAGIC_SQRT_2: u128 = 0xfff97272373d413259a46990580e213a;
pub const MAGIC_SQRT_4: u128 = 0xfff2e50f5f656932ef12357cf3c7fdcc;
pub const MAGIC_SQRT_8: u128 = 0xffe5caca7e10e4e61c3624eaa0941cd0;
pub const MAGIC_SQRT_16: u128 = 0xffcb9843d60f6159c9db58835c926644;
pub const MAGIC_SQRT_32: u128 = 0xff973b41fa98c081472e6896dfb254c0;
pub const MAGIC_SQRT_64: u128 = 0xff2ea16466c96a3843ec78b326b52861;
pub const MAGIC_SQRT_128: u128 = 0xfe5dee046a99a2a811c461f1969c3053;
pub const MAGIC_SQRT_256: u128 = 0xfcbe86c7900a88aedcffc83b479aa3a4;
pub const MAGIC_SQRT_512: u128 = 0xf987a7253ac413176f2b074cf7815e54;
pub const MAGIC_SQRT_1024: u128 = 0xf3392b0822b70005940c7a398e4b70f3;
pub const MAGIC_SQRT_2048: u128 = 0xe7159475a2c29b7443b29c7fa6e889d9;
pub const MAGIC_SQRT_4096: u128 = 0xd097f3bdfd2022b8845ad8f792aa5825;
pub const MAGIC_SQRT_8192: u128 = 0xa9f746462d870fdf8a65dc1f90e061e5;
pub const MAGIC_SQRT_16384: u128 = 0x70d869a156d2a1b890bb3df62baf32f7;
pub const MAGIC_SQRT_32768: u128 = 0x31be135f97d08fd981231505542fcfa6;
pub const MAGIC_SQRT_65536: u128 = 0x9aa508b5b7a84e1c677de54f3e99bc9;
pub const MAGIC_SQRT_131072: u128 = 0x5d6af8dedb81196699c329225ee604;
pub const MAGIC_SQRT_262144: u128 = 0x2216e584f5fa1ea926041bedfe98;
pub const MAGIC_SQRT_524288: u128 = 0x48a170391f7dc42444e8fa2;
