use assert_matches::*;
use trdelnik_client::fuzzing::*;

mod program_stubs;

use program_stubs::*;

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
}

fn main() {
    test_syscall_stubs();
    loop {
        fuzz!(|fuzz_data: FuzzData| {
            // let alice_account_size = std::mem::size_of::<your_program::AliceAccount>();
            let mut alice_account =
                NativeAccountData::new(8, PROGRAM_ID, true, true, false, 5 * LAMPORTS_PER_SOL);

            let mut system_program = create_program_account(system_program::ID);
            let mut token_program = create_program_account(spl_token::id());

            let account_infos = [
                alice_account.as_account_info(),
                system_program.as_account_info(),
                token_program.as_account_info(),
            ];
            run_dummy_ix1(&account_infos, &fuzz_data);
        });
    }
}

fn run_dummy_ix1(account_infos: &[AccountInfo], fuzz_data: &FuzzData) {
    let dummy_data = your_program::instruction::YourInstruction {
        data_field1: fuzz_data.param1,
    }
    .data();
    let res = entry(&PROGRAM_ID, account_infos, &dummy_data);
    assert_matches!(res, Ok(()));
}
