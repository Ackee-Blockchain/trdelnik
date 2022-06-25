// DO NOT EDIT - automatically generated file
pub mod escrow_instruction {
    use trdelnik_client::*;
    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        5u8, 214u8, 204u8, 101u8, 166u8, 163u8, 239u8, 244u8, 13u8, 110u8, 64u8, 106u8, 230u8,
        81u8, 141u8, 186u8, 208u8, 155u8, 78u8, 83u8, 194u8, 215u8, 103u8, 17u8, 94u8, 15u8, 137u8,
        68u8, 170u8, 153u8, 74u8, 59u8,
    ]);
    pub async fn initialize_escrow(
        client: &Client,
        i_initializer_amount: u64,
        i_taker_amount: u64,
        a_initializer: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_system_program: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransaction, ClientError> {
        Ok(client
            .send_instruction(
                PROGRAM_ID,
                escrow::instruction::InitializeEscrow {
                    initializer_amount: i_initializer_amount,
                    taker_amount: i_taker_amount,
                },
                escrow::accounts::InitializeEscrow {
                    initializer: a_initializer,
                    initializer_deposit_token_account: a_initializer_deposit_token_account,
                    initializer_receive_token_account: a_initializer_receive_token_account,
                    escrow_account: a_escrow_account,
                    system_program: a_system_program,
                    token_program: a_token_program,
                },
                signers,
            )
            .await?)
    }
    pub fn initialize_escrow_ix(
        i_initializer_amount: u64,
        i_taker_amount: u64,
        a_initializer: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_system_program: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: escrow::instruction::InitializeEscrow {
                initializer_amount: i_initializer_amount,
                taker_amount: i_taker_amount,
            }
            .data(),
            accounts: escrow::accounts::InitializeEscrow {
                initializer: a_initializer,
                initializer_deposit_token_account: a_initializer_deposit_token_account,
                initializer_receive_token_account: a_initializer_receive_token_account,
                escrow_account: a_escrow_account,
                system_program: a_system_program,
                token_program: a_token_program,
            }
            .to_account_metas(None),
        }
    }
    pub async fn cancel_escrow(
        client: &Client,
        a_initializer: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransaction, ClientError> {
        Ok(client
            .send_instruction(
                PROGRAM_ID,
                escrow::instruction::CancelEscrow {},
                escrow::accounts::CancelEscrow {
                    initializer: a_initializer,
                    pda_deposit_token_account: a_pda_deposit_token_account,
                    pda_account: a_pda_account,
                    escrow_account: a_escrow_account,
                    token_program: a_token_program,
                },
                signers,
            )
            .await?)
    }
    pub fn cancel_escrow_ix(
        a_initializer: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: escrow::instruction::CancelEscrow {}.data(),
            accounts: escrow::accounts::CancelEscrow {
                initializer: a_initializer,
                pda_deposit_token_account: a_pda_deposit_token_account,
                pda_account: a_pda_account,
                escrow_account: a_escrow_account,
                token_program: a_token_program,
            }
            .to_account_metas(None),
        }
    }
    pub async fn exchange(
        client: &Client,
        a_taker: anchor_lang::solana_program::pubkey::Pubkey,
        a_taker_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_taker_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_main_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransaction, ClientError> {
        Ok(client
            .send_instruction(
                PROGRAM_ID,
                escrow::instruction::Exchange {},
                escrow::accounts::Exchange {
                    taker: a_taker,
                    taker_deposit_token_account: a_taker_deposit_token_account,
                    taker_receive_token_account: a_taker_receive_token_account,
                    pda_deposit_token_account: a_pda_deposit_token_account,
                    initializer_receive_token_account: a_initializer_receive_token_account,
                    initializer_main_account: a_initializer_main_account,
                    escrow_account: a_escrow_account,
                    pda_account: a_pda_account,
                    token_program: a_token_program,
                },
                signers,
            )
            .await?)
    }
    pub fn exchange_ix(
        a_taker: anchor_lang::solana_program::pubkey::Pubkey,
        a_taker_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_taker_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_deposit_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_receive_token_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_initializer_main_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_escrow_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_pda_account: anchor_lang::solana_program::pubkey::Pubkey,
        a_token_program: anchor_lang::solana_program::pubkey::Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: escrow::instruction::Exchange {}.data(),
            accounts: escrow::accounts::Exchange {
                taker: a_taker,
                taker_deposit_token_account: a_taker_deposit_token_account,
                taker_receive_token_account: a_taker_receive_token_account,
                pda_deposit_token_account: a_pda_deposit_token_account,
                initializer_receive_token_account: a_initializer_receive_token_account,
                initializer_main_account: a_initializer_main_account,
                escrow_account: a_escrow_account,
                pda_account: a_pda_account,
                token_program: a_token_program,
            }
            .to_account_metas(None),
        }
    }
}
