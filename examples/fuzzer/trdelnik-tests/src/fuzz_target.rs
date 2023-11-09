use assert_matches::*;
use fuzzer::entry;
use program_client::fuzzer_instruction::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::{program_stubs, system_program};
use trdelnik_client::anchor_lang::Discriminator;
use trdelnik_client::fuzzing::*;
use trdelnik_tests::native_account_data::*;

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
}

fn main() {
    test_syscall_stubs();

    loop {
        fuzz!(|fuzz_data: FuzzData| {
            let mut counter = NativeAccountData::new(
                (8 + 40) as usize,
                PROGRAM_ID,
                true,
                true,
                false,
                5 * LAMPORTS_PER_SOL,
            );
            let mut user = NativeAccountData::new(
                0,
                system_program::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            let mut system_program = create_program_account(SYSTEM_PROGRAM_ID);

            let accounts = [
                counter.as_account_info(),
                user.as_account_info(),
                system_program.as_account_info(),
            ];

            let mut account_data = accounts
                .iter()
                .map(NativeAccountData::new_from_account_info)
                .collect::<Vec<_>>();
            let account_infos = account_data
                .iter_mut()
                .map(NativeAccountData::as_account_info)
                .collect::<Vec<_>>();

            let data: [u8; 8] = fuzzer::instruction::Initialize::DISCRIMINATOR;
            let res = entry(&PROGRAM_ID, &account_infos, &data);

            assert_matches!(res, Ok(()));

            let mut data: [u8; 10] = [0u8; 10];
            data[..8].copy_from_slice(&fuzzer::instruction::Update::DISCRIMINATOR);
            data[8] = fuzz_data.param1;
            data[9] = fuzz_data.param2;

            let res = entry(&PROGRAM_ID, &account_infos, &data);
            assert_matches!(res, Ok(()));
        });
    }
}

struct TestSyscallStubs {}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
}

fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
    });
}
