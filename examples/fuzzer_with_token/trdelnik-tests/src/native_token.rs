use crate::NativeAccountData;

use solana_program::{program_pack::Pack, pubkey::Pubkey};
use spl_token::state::{Account as TokenAccount, AccountState as TokenAccountState, Mint};

pub fn create_token_account(
    mint_account: &mut NativeAccountData,
    account_data: &mut NativeAccountData,
    owner: &Pubkey,
    amount: u64,
) {
    let mut mint = Mint::unpack(&mint_account.data).unwrap();
    let account = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: mint_account.key,
        owner: *owner,
        amount,
        ..Default::default()
    };
    mint.supply += amount;
    Mint::pack(mint, &mut mint_account.data[..]).unwrap();
    TokenAccount::pack(account, &mut account_data.data[..]).unwrap();
}
