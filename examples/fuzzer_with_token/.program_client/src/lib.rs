// DO NOT EDIT - automatically generated file (except `use` statements inside the `*_instruction` module
pub mod fuzzer_with_token_instruction {
    use trdelnik_client::*;
    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        156u8, 140u8, 147u8, 248u8, 22u8, 156u8, 142u8, 123u8, 180u8, 128u8, 136u8, 164u8, 202u8,
        180u8, 206u8, 125u8, 38u8, 71u8, 9u8, 246u8, 16u8, 33u8, 175u8, 57u8, 180u8, 1u8, 122u8,
        221u8, 121u8, 179u8, 169u8, 159u8,
    ]);
    pub async fn initialize(
        client: &Client,
        i_transfer_amount: u8,
        a_counter: Pubkey,
        a_user_a: Pubkey,
        a_user_b: Pubkey,
        a_user_a_token_account: Pubkey,
        a_user_b_token_account: Pubkey,
        a_mint: Pubkey,
        a_system_program: Pubkey,
        a_token_program: Pubkey,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        client
            .send_instruction(
                PROGRAM_ID,
                fuzzer_with_token::instruction::Initialize {
                    transfer_amount: i_transfer_amount,
                },
                fuzzer_with_token::accounts::Initialize {
                    counter: a_counter,
                    user_a: a_user_a,
                    user_b: a_user_b,
                    user_a_token_account: a_user_a_token_account,
                    user_b_token_account: a_user_b_token_account,
                    mint: a_mint,
                    system_program: a_system_program,
                    token_program: a_token_program,
                },
                signers,
            )
            .await
    }
    pub fn initialize_ix(
        i_transfer_amount: u8,
        a_counter: Pubkey,
        a_user_a: Pubkey,
        a_user_b: Pubkey,
        a_user_a_token_account: Pubkey,
        a_user_b_token_account: Pubkey,
        a_mint: Pubkey,
        a_system_program: Pubkey,
        a_token_program: Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: fuzzer_with_token::instruction::Initialize {
                transfer_amount: i_transfer_amount,
            }
            .data(),
            accounts: fuzzer_with_token::accounts::Initialize {
                counter: a_counter,
                user_a: a_user_a,
                user_b: a_user_b,
                user_a_token_account: a_user_a_token_account,
                user_b_token_account: a_user_b_token_account,
                mint: a_mint,
                system_program: a_system_program,
                token_program: a_token_program,
            }
            .to_account_metas(None),
        }
    }
    pub async fn update(
        client: &Client,
        i_input1: u8,
        i_input2: u8,
        a_counter: Pubkey,
        a_authority: Pubkey,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        client
            .send_instruction(
                PROGRAM_ID,
                fuzzer_with_token::instruction::Update {
                    input1: i_input1,
                    input2: i_input2,
                },
                fuzzer_with_token::accounts::Update {
                    counter: a_counter,
                    authority: a_authority,
                },
                signers,
            )
            .await
    }
    pub fn update_ix(
        i_input1: u8,
        i_input2: u8,
        a_counter: Pubkey,
        a_authority: Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: fuzzer_with_token::instruction::Update {
                input1: i_input1,
                input2: i_input2,
            }
            .data(),
            accounts: fuzzer_with_token::accounts::Update {
                counter: a_counter,
                authority: a_authority,
            }
            .to_account_metas(None),
        }
    }
}
