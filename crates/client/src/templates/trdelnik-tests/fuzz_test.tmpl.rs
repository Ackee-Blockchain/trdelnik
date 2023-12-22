use assert_matches::*;
use trdelnik_client::fuzzing::*;

const PROGRAM_NAME: &str = "###PROGRAM_NAME###";

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
}

fn main() {
    loop {
        fuzz!(|fuzz_data: FuzzData| {
            solana_program_test::tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    let program_test = solana_program_test::ProgramTest::new(
                        PROGRAM_NAME,
                        PROGRAM_ID,
                        solana_program_test::processor!(entry),
                    );

                    let mut ctx = program_test.start_with_context().await;

                    // TODO: replace this instruction with one of your generated instructions from trdelnik_client
                    let init_ix = init_dummy_ix();
                    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
                        &[init_ix],
                        Some(&ctx.payer.pubkey()),
                    );

                    transaction.sign(&[&ctx.payer], ctx.last_blockhash);
                    let res = ctx.banks_client.process_transaction(transaction).await;
                    assert_matches!(res, Ok(()));

                    let res = fuzz_ix(
                        &fuzz_data,
                        &mut ctx.banks_client,
                        &ctx.payer,
                        ctx.last_blockhash,
                    )
                    .await;
                    assert_matches!(res, Ok(()));
                });
        });
    }
}

async fn fuzz_ix(
    fuzz_data: &FuzzData,
    banks_client: &mut solana_program_test::BanksClient,
    payer: &solana_sdk::signature::Keypair,
    blockhash: solana_sdk::hash::Hash,
) -> core::result::Result<(), solana_program_test::BanksClientError> {
    // TODO: replace this instruction with one of your generated instructions from trdelnik_client
    let update_ix = update_dummy_ix(fuzz_data.param1, fuzz_data.param2);

    let mut transaction =
        solana_sdk::transaction::Transaction::new_with_payer(&[update_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], blockhash);

    banks_client.process_transaction(transaction).await
}

fn init_dummy_ix() -> solana_sdk::instruction::Instruction {
    solana_sdk::instruction::Instruction {
        program_id: PROGRAM_ID,
        data: vec![],
        accounts: vec![],
    }
}

fn update_dummy_ix(param1: u8, param2: u8) -> solana_sdk::instruction::Instruction {
    solana_sdk::instruction::Instruction {
        program_id: PROGRAM_ID,
        data: vec![param1, param2],
        accounts: vec![],
    }
}
