use assert_matches::*;
use fuzzer::entry;
use program_client::fuzzer_instruction::*;
use trdelnik_client::fuzzing::*;

const PROGRAM_NAME: &str = "fuzzer";

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

                    let counter = solana_sdk::signature::Keypair::new();

                    let init_ix = initialize_ix(
                        &counter.pubkey(),
                        &ctx.payer.pubkey(),
                        &solana_sdk::system_program::ID,
                    );
                    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
                        &[init_ix],
                        Some(&ctx.payer.pubkey()),
                    );

                    transaction.sign(&[&ctx.payer, &counter], ctx.last_blockhash);
                    let res = ctx.banks_client.process_transaction(transaction).await;
                    assert_matches!(res, Ok(()));

                    let res = fuzz_update_ix(
                        &fuzz_data,
                        &mut ctx.banks_client,
                        &ctx.payer,
                        &counter,
                        ctx.last_blockhash,
                    )
                    .await;
                    assert_matches!(res, Ok(()));
                });
        });
    }
}

async fn fuzz_update_ix(
    fuzz_data: &FuzzData,
    banks_client: &mut solana_program_test::BanksClient,
    payer: &solana_sdk::signature::Keypair,
    counter: &solana_sdk::signature::Keypair,
    blockhash: solana_sdk::hash::Hash,
) -> core::result::Result<(), solana_program_test::BanksClientError> {
    let update_ix = update_ix(
        fuzz_data.param1,
        fuzz_data.param2,
        &counter.pubkey(),
        &payer.pubkey(),
    );

    let mut transaction =
        solana_sdk::transaction::Transaction::new_with_payer(&[update_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], blockhash);

    banks_client.process_transaction(transaction).await
}
