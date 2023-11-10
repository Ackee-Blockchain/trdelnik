use crate::NativeAccountData;

use solana_program::{program_option::COption, program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Mint;

pub fn create_mint(owner: &Pubkey, account_data: &mut NativeAccountData) {
    let mint = Mint {
        is_initialized: true,
        mint_authority: COption::Some(*owner),
        ..Default::default()
    };
    Mint::pack(mint, &mut account_data.data[..]).unwrap();
}
