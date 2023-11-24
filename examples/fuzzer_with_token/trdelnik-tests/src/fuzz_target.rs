use assert_matches::*;
use fuzzer_with_token::entry;
use program_client::fuzzer_with_token_instruction::*;

use spl_token::state::{Account as TokenAccount, Mint};
use trdelnik_client::fuzzing::*;
use trdelnik_tests::syscall_stubs::*;

// I think we cannot intercept system program instructions such as create, allocate, assign from
// https://github.com/solana-labs/solana/blob/b5256997f8d86c9bfbfa7467ba8a1f72140d4bd8/programs/system/src/system_processor.rs
// so we need to defined this functionality within native_account_data

const INITIAL_AMOUNT_USER_A: u64 = LAMPORTS_PER_SOL * 1000;

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
}

fn main() {
    test_syscall_stubs();
    loop {
        fuzz!(|fuzz_data: FuzzData| {
            let counter_len = std::mem::size_of::<fuzzer_with_token::Counter>();
            let mut counter = NativeAccountData::new(
                8 + counter_len,
                PROGRAM_ID,
                true,
                true,
                false,
                5 * LAMPORTS_PER_SOL,
            );
            let mut user_a = NativeAccountData::new(
                0,
                system_program::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            let mut user_b = NativeAccountData::new(
                0,
                system_program::id(),
                false,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );

            let mut mint = NativeAccountData::new(
                Mint::LEN,
                spl_token::id(),
                false,
                false,
                false,
                LAMPORTS_PER_SOL * 5,
            );

            //create_mint(&user_a.key, &mut mint);

            // user_a token account which is expected initialized
            let mut token_a = NativeAccountData::new(
                TokenAccount::LEN,
                spl_token::id(),
                true,
                false,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            //create_token_account(&mut mint, &mut token_a, &user_a.key, INITIAL_AMOUNT_USER_A);

            // user_b token account which is expected not initialized
            let mut token_b = NativeAccountData::new(
                TokenAccount::LEN,
                spl_token::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );

            let mut system_program = create_program_account(system_program::ID);
            let mut token_program = create_program_account(spl_token::id());

            let account_infos = [
                counter.as_account_info(),
                user_a.as_account_info(),
                user_b.as_account_info(),
                token_a.as_account_info(),
                token_b.as_account_info(),
                mint.as_account_info(),
                system_program.as_account_info(),
                token_program.as_account_info(),
            ];

            //let account_infos = to_account_info(&mut account_infos);
            run_initialize_instr(&account_infos, &fuzz_data);

            let account_infos = [counter.as_account_info(), user_a.as_account_info()];

            run_update_instr(&account_infos, &fuzz_data);
        });
    }
}

//this main is used for testing purpose
// fn main() {
//     test_syscall_stubs();
//     // Data setup this way will ensure that create_account/transfer/allocate ...
//     // do not have to be called = no invoke_signed invoked
//     let counter_len = std::mem::size_of::<fuzzer_with_token::Counter>();
//     let mut counter = NativeAccountData::new(
//         8 + counter_len,
//         PROGRAM_ID,
//         true,
//         true,
//         false,
//         5 * LAMPORTS_PER_SOL,
//     );
//     let mut user_a = NativeAccountData::new(
//         0,
//         system_program::id(),
//         true,
//         true,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );
//     let mut user_b = NativeAccountData::new(
//         0,
//         system_program::id(),
//         false,
//         true,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );

//     let mut mint = NativeAccountData::new(
//         Mint::LEN,
//         spl_token::id(),
//         false,
//         false,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );

//     //create_mint(&user_a.key, &mut mint);

//     // user_a token account which is expected initialized
//     let mut token_a = NativeAccountData::new(
//         TokenAccount::LEN,
//         spl_token::id(),
//         true,
//         false,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );
//     //create_token_account(&mut mint, &mut token_a, &user_a.key, INITIAL_AMOUNT_USER_A);

//     // user_b token account which is expected not initialized
//     let mut token_b = NativeAccountData::new(
//         TokenAccount::LEN,
//         spl_token::id(),
//         true,
//         true,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );

//     let mut system_program = create_program_account(system_program::ID);
//     let mut token_program = create_program_account(spl_token::id());

//     let account_infos = [
//         counter.as_account_info(),
//         user_a.as_account_info(),
//         user_b.as_account_info(),
//         token_a.as_account_info(),
//         token_b.as_account_info(),
//         mint.as_account_info(),
//         system_program.as_account_info(),
//         token_program.as_account_info(),
//     ];

//     //let account_infos = to_account_info(&mut account_infos);
//     run_initialize_instr(&account_infos);

//     let account_infos = [counter.as_account_info(), user_a.as_account_info()];
//     run_update_instr(&account_infos);
// }

fn run_initialize_instr(account_infos: &[AccountInfo], fuzz_data: &FuzzData) {
    let initialize_data = fuzzer_with_token::instruction::Initialize {
        transfer_amount: fuzz_data.param2,
    };
    let initialize_instr_data = initialize_data.data();
    let res = entry(&PROGRAM_ID, account_infos, &initialize_instr_data);
    assert_matches!(res, Ok(()));

    let counter = NativeAccountData::new_from_account_info(&account_infos[0]);
    let token_a = NativeAccountData::new_from_account_info(&account_infos[3]);
    let token_b = NativeAccountData::new_from_account_info(&account_infos[4]);

    let counter_data =
        fuzzer_with_token::Counter::try_deserialize(&mut counter.data.as_slice()).unwrap();

    let token_a_data = TokenAccount::unpack(token_a.data.as_slice()).unwrap();
    let token_b_data = TokenAccount::unpack(token_b.data.as_slice()).unwrap();

    // Run checks
    assert_eq!(counter_data.count, 1);
    assert_eq!(
        token_a_data.amount,
        INITIAL_AMOUNT_USER_A - fuzz_data.param2 as u64
    );
    assert_eq!(token_b_data.amount, fuzz_data.param2 as u64);
}

fn run_update_instr(account_infos: &[AccountInfo], fuzz_data: &FuzzData) {
    let update_data = fuzzer_with_token::instruction::Update {
        input1: fuzz_data.param1,
        input2: fuzz_data.param2,
    };
    let counter = NativeAccountData::new_from_account_info(&account_infos[0]);
    let counter_data =
        fuzzer_with_token::Counter::try_deserialize(&mut counter.data.as_slice()).unwrap();
    assert_eq!(counter_data.count, 1);

    let update_instr_data = update_data.data();
    let res = entry(&PROGRAM_ID, account_infos, &update_instr_data);
    assert_matches!(res, Ok(()));

    let counter = NativeAccountData::new_from_account_info(&account_infos[0]);
    let counter_data =
        fuzzer_with_token::Counter::try_deserialize(&mut counter.data.as_slice()).unwrap();

    // Run checks
    assert_eq!(
        counter_data.count,
        fuzzer_with_token::buggy_math_function(fuzz_data.param1, fuzz_data.param2) as u64
    );
}
