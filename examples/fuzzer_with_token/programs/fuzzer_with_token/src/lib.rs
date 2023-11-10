use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

const MAGIC_NUMBER: u8 = 254;

declare_id!("BY6vsKLyMNhcSLEhaD4iUc2B49PjQ7wkqB4CeoT4eca2");

#[program]
pub mod fuzzer_with_token {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, transfer_amount: u8) -> Result<()> {
        let counter = &mut ctx.accounts.counter;

        counter.count = 0;
        counter.authority = ctx.accounts.user_a.key();
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_a_token_account.to_account_info(),
                    to: ctx.accounts.user_b_token_account.to_account_info(),
                    authority: ctx.accounts.user_a.to_account_info(),
                },
            ),
            transfer_amount as u64,
        )?;

        Ok(())
    }

    pub fn update(ctx: Context<Update>, input1: u8, input2: u8) -> Result<()> {
        let counter = &mut ctx.accounts.counter;

        msg!("input1 = {}, input2 = {}", input1, input2);

        // comment this to fix the black magic panic
        if input1 == MAGIC_NUMBER {
            panic!("Black magic not supported!");
        }

        counter.count = buggy_math_function(input1, input2).into();
        Ok(())
    }
}

pub fn buggy_math_function(input1: u8, input2: u8) -> u8 {
    // comment the if statement to cause div-by-zero and subtract with overflow panic
    if input2 >= MAGIC_NUMBER {
        return 0;
    }
    let divisor = MAGIC_NUMBER - input2;
    input1 / divisor
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user_a, space = 8 + 40)]
    pub counter: Account<'info, Counter>,

    #[account(mut)]
    pub user_a: Signer<'info>,

    /// CHECK: OK
    pub user_b: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = user_a
    )]
    pub user_a_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        token::mint = mint,
        token::authority = user_b,
        payer = user_a
    )]
    pub user_b_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}
