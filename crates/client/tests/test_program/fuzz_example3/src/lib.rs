use anchor_lang::prelude::*;

mod error;
mod instructions;
pub mod state;

use crate::instructions::*;
pub use error::*;
declare_id!("FzwXe6bxBZXcqmkdv37YvFARRvqh2JvPW8RrLiTrZTtP");

#[program]
pub mod fuzz_example3 {
    use super::*;

    pub fn init_vesting(
        ctx: Context<InitVesting>,
        i_recipient: Pubkey,
        amount: u64,
        start_at: u64,
        end_at: u64,
        interval: u64,
    ) -> Result<()> {
        _init_vesting(ctx, i_recipient, amount, start_at, end_at, interval)
    }

    pub fn withdraw_unlocked(ctx: Context<WithdrawUnlocked>) -> Result<()> {
        _withdraw_unlocked(ctx)
    }
}
