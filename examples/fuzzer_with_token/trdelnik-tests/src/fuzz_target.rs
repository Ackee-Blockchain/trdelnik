use assert_matches::*;
use fuzzer_with_token::entry;
use program_client::fuzzer_with_token_instruction::*;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, native_token::LAMPORTS_PER_SOL,
    program_pack::Pack, program_stubs, system_program,
};
use spl_token::state::{Account as TokenAccount, Mint};
use trdelnik_client::{
    anchor_lang::{AccountDeserialize, Discriminator},
    fuzzing::*,
};
use trdelnik_tests::{
    native_account_data::*, native_mint::create_mint, native_token::create_token_account,
};

// I think we cannot intercept system program instructions such as create, allocate, assign from
// https://github.com/solana-labs/solana/blob/b5256997f8d86c9bfbfa7467ba8a1f72140d4bd8/programs/system/src/system_processor.rs
// so we need to defined this functionality within native_account_data

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
}

fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
    });
}

struct TestSyscallStubs {}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        _signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        if instruction.program_id == spl_token::id() {
            spl_token::processor::Processor::process(
                &instruction.program_id,
                account_infos,
                &instruction.data,
            )
        } else {
            self.sol_log("SyscallStubs: sol_invoke_signed() not available");
            Ok(())
        }
    }
    fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
}

fn main() {
    test_syscall_stubs();
    loop {
        fuzz!(|fuzz_data: FuzzData| {
            // Data setup this way will ensure that create_account/transfer/allocate ...
            // do not have to be called = no invoke_signed invoked
            let counter = NativeAccountData::new(
                (8 + 40) as usize,
                PROGRAM_ID,
                true,
                true,
                false,
                5 * LAMPORTS_PER_SOL,
            );
            let user_a = NativeAccountData::new(
                0,
                system_program::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            let user_b = NativeAccountData::new(
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

            create_mint(&user_a.key, &mut mint);

            // user_a token account which is expected initialized
            let mut token_a = NativeAccountData::new(
                TokenAccount::LEN,
                spl_token::id(),
                true,
                false,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            const INITIAL_AMOUNT_USER_A: u64 = LAMPORTS_PER_SOL * 58;
            create_token_account(&mut mint, &mut token_a, &user_a.key, INITIAL_AMOUNT_USER_A);

            // user_b token account which is expected not initialized
            let token_b = NativeAccountData::new(
                TokenAccount::LEN,
                spl_token::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );

            let system_program = create_program_account(SYSTEM_PROGRAM_ID);
            let token_program = create_program_account(spl_token::id());

            let mut accounts = [
                counter,
                user_a,
                user_b,
                token_a,
                token_b,
                mint,
                system_program,
                token_program,
            ];

            let account_infos = accounts
                .iter_mut()
                .map(NativeAccountData::as_account_info)
                .collect::<Vec<_>>();

            // **********************************************************************************
            // Initialize instr
            let mut data: [u8; 9] = [0u8; 9];
            // first 8 bytes for instruction
            data[..8].copy_from_slice(&fuzzer_with_token::instruction::Initialize::DISCRIMINATOR);
            // lat byte for instruction input = amount that will be transfered from A to B
            data[8] = fuzz_data.param1;
            let res = entry(&PROGRAM_ID, &account_infos, &data);
            assert_matches!(res, Ok(()));

            // **********************************************************************************
            // Deserialize Data
            let counter = NativeAccountData::new_from_account_info(&account_infos[0]);
            let token_a = NativeAccountData::new_from_account_info(&account_infos[3]);
            let token_b = NativeAccountData::new_from_account_info(&account_infos[4]);

            let counter_data =
                fuzzer_with_token::Counter::try_deserialize(&mut counter.data.as_slice()).unwrap();

            let token_a_data = TokenAccount::unpack(token_a.data.as_slice()).unwrap();
            let token_b_data = TokenAccount::unpack(token_b.data.as_slice()).unwrap();

            // Counter initialized to 1
            assert_eq!(counter_data.count, 1);
            // User A sent tokens
            assert_eq!(
                token_a_data.amount,
                INITIAL_AMOUNT_USER_A - fuzz_data.param1 as u64
            );
            // User B received tokens
            assert_eq!(token_b_data.amount, fuzz_data.param1 as u64);

            // Create Instr Data for Update instruction
            let mut data: [u8; 10] = [0u8; 10];
            data[..8].copy_from_slice(&fuzzer_with_token::instruction::Update::DISCRIMINATOR);
            data[8] = fuzz_data.param1;
            data[9] = fuzz_data.param2;

            let res = entry(&PROGRAM_ID, &account_infos, &data);
            assert_eq!(res, Ok(()));
        });
    }
}

//this main is used for testing purpose
// fn main() {
//     test_syscall_stubs();

//     // Data setup this way will ensure that create_account/transfer/allocate ...
//     // do not have to be called = no invoke_signed invoked
//     let counter = NativeAccountData::new(
//         (8 + 40) as usize,
//         PROGRAM_ID,
//         true,
//         true,
//         false,
//         5 * LAMPORTS_PER_SOL,
//     );
//     let user_a = NativeAccountData::new(
//         0,
//         system_program::id(),
//         true,
//         true,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );
//     let user_b = NativeAccountData::new(
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

//     create_mint(&user_a.key, &mut mint);

//     // user_a token account which is expected initialized
//     let mut token_a = NativeAccountData::new(
//         TokenAccount::LEN,
//         spl_token::id(),
//         true,
//         false,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );
//     const INITIAL_AMOUNT_USER_A: u64 = LAMPORTS_PER_SOL * 58;
//     create_token_account(&mut mint, &mut token_a, &user_a.key, INITIAL_AMOUNT_USER_A);

//     // user_b token account which is expected not initialized
//     let token_b = NativeAccountData::new(
//         TokenAccount::LEN,
//         spl_token::id(),
//         true,
//         true,
//         false,
//         LAMPORTS_PER_SOL * 5,
//     );

//     let system_program = create_program_account(SYSTEM_PROGRAM_ID);
//     let token_program = create_program_account(spl_token::id());

//     let mut accounts = [
//         counter,
//         user_a,
//         user_b,
//         token_a,
//         token_b,
//         mint,
//         system_program,
//         token_program,
//     ];

//     let account_infos = accounts
//         .iter_mut()
//         .map(NativeAccountData::as_account_info)
//         .collect::<Vec<_>>();

//     // **********************************************************************************
//     // Initialize instr
//     let mut data: [u8; 9] = [0u8; 9];
//     // first 8 bytes for instruction
//     data[..8].copy_from_slice(&fuzzer_with_token::instruction::Initialize::DISCRIMINATOR);
//     // lat byte for instruction input = amount that will be transfered from A to B
//     data[8] = 5;
//     let res = entry(&PROGRAM_ID, &account_infos, &data);
//     assert_matches!(res, Ok(()));

//     // **********************************************************************************
//     // Deserialize Data
//     let counter = NativeAccountData::new_from_account_info(&account_infos[0]);
//     let token_a = NativeAccountData::new_from_account_info(&account_infos[3]);
//     let token_b = NativeAccountData::new_from_account_info(&account_infos[4]);

//     let counter_data =
//         fuzzer_with_token::Counter::try_deserialize(&mut counter.data.as_slice()).unwrap();

//     let token_a_data = TokenAccount::unpack(token_a.data.as_slice()).unwrap();
//     let token_b_data = TokenAccount::unpack(token_b.data.as_slice()).unwrap();

//     // Counter initialized to 1
//     assert_eq!(counter_data.count, 1);
//     // User A sent tokens
//     assert_eq!(token_a_data.amount, INITIAL_AMOUNT_USER_A - 5 as u64);
//     // User B received tokens
//     assert_eq!(token_b_data.amount, 5 as u64);

//     // Create Instr Data for Update instruction
//     let mut data: [u8; 10] = [0u8; 10];
//     data[..8].copy_from_slice(&fuzzer_with_token::instruction::Update::DISCRIMINATOR);
//     data[8] = 51;
//     data[9] = 14;

//     let res = entry(&PROGRAM_ID, &account_infos, &data);
//     assert_eq!(res, Ok(()));
// }
