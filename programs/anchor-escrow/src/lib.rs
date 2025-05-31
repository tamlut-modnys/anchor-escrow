use anchor_lang::prelude::*;

pub mod contexts;
// private export to get contexts module usable in this file
use contexts::*;

pub mod state;
// public re-export to get the state module and also make it available publicly
pub use state::*;

declare_id!("6BLPdL9narQPFQsqS7AXuRBRS4VoyKmHHzdwkgnLaAps");

#[program]
pub mod anchor_escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        ctx.accounts.deposit(deposit)?;
        ctx.accounts.save_escrow(seed, receive, &ctx.bumps)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit()?;
        ctx.accounts.withdraw_and_close_vault()
    }
}
