use anchor_lang::prelude::*;

#[error_code]
pub enum ClammError {
    #[msg("Invalid tick: must be within bounds and divisible by tick spacing")]
    InvalidTick,

    #[msg("Invalid tick range: lower must be less than upper")]
    InvalidTickRange,

    #[msg("Invalid sqrt price: must be within bounds")]
    InvalidSqrtPrice,

    #[msg("Invalid fee rate: exceeds maximum")]
    InvalidFeeRate,

    #[msg("Invalid tick spacing")]
    InvalidTickSpacing,

    #[msg("Invalid tick array: start index not aligned")]
    InvalidTickArrayStart,

    #[msg("Tick not found in provided tick array")]
    TickNotInArray,

    #[msg("Liquidity overflow")]
    LiquidityOverflow,

    #[msg("Liquidity underflow")]
    LiquidityUnderflow,

    #[msg("Token amount overflow")]
    TokenAmountOverflow,

    #[msg("Insufficient output amount (slippage exceeded)")]
    SlippageExceeded,

    #[msg("Pool has zero liquidity")]
    ZeroLiquidity,

    #[msg("Position has zero liquidity")]
    PositionZeroLiquidity,

    #[msg("Invalid token order: mint_x must be less than mint_y")]
    InvalidTokenOrder,

    #[msg("Sqrt price out of bounds for tick")]
    SqrtPriceOutOfBounds,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid token amounts: both must be greater than zero")]
    InvalidTokenAmounts,

    #[msg("Cannot close position with remaining liquidity")]
    PositionHasLiquidity,

    #[msg("Cannot close position with uncollected fees")]
    PositionHasUncollectedFees,
}
