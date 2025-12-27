use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, CloseAccount, Mint, Token, TokenAccount};
use crate::state::Position;
use crate::errors::ClammError;

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(
        mut,
        close = receiver,
        seeds = [b"position", position_mint.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        constraint = position_mint.key() == position.position_mint
    )]
    pub position_mint: Account<'info, Mint>,

    /// Token account holding the position NFT
    #[account(
        mut,
        constraint = position_token_account.mint == position.position_mint,
        constraint = position_token_account.amount == 1,
        constraint = position_token_account.owner == owner.key()
    )]
    pub position_token_account: Account<'info, TokenAccount>,

    pub owner: Signer<'info>,

    /// CHECK: Receives the rent from closed accounts
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ClosePosition<'info> {
    pub fn validate(&self) -> Result<()> {
        require!(
            self.position.liquidity == 0,
            ClammError::PositionHasLiquidity
        );
        require!(
            self.position.tokens_owed_x == 0 && self.position.tokens_owed_y == 0,
            ClammError::PositionHasUncollectedFees
        );
        Ok(())
    }

    pub fn burn_position_nft(&self) -> Result<()> {
        token::burn(
            CpiContext::new(
                self.token_program.to_account_info(),
                Burn {
                    mint: self.position_mint.to_account_info(),
                    from: self.position_token_account.to_account_info(),
                    authority: self.owner.to_account_info(),
                },
            ),
            1,
        )
    }

    pub fn close_token_account(&self) -> Result<()> {
        token::close_account(CpiContext::new(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.position_token_account.to_account_info(),
                destination: self.receiver.to_account_info(),
                authority: self.owner.to_account_info(),
            },
        ))
    }
}

/// Close a position and reclaim rent
pub fn close_position_handler(ctx: Context<ClosePosition>) -> Result<()> {
    // Validate if we can close the position
    ctx.accounts.validate()?;

    // Burn the nft associated with that position
    ctx.accounts.burn_position_nft()?;

    // Close the Token Account since now it's empty
    ctx.accounts.close_token_account()?;

    Ok(())
}
