use assert_matches::*;
use fuzzer::entry;
use program_client::fuzzer_instruction::*;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::{program_stubs, system_program};
use trdelnik_client::anchor_lang::Discriminator;
use trdelnik_client::fuzzing::*;
use trdelnik_tests::native_account_data::*;

// use bincode::deserialize;
// use serde::Serialize;
// use solana_program::system_instruction::SystemInstruction;

#[derive(Arbitrary)]
pub struct FuzzData {
    param1: u8,
    param2: u8,
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
            let user = NativeAccountData::new(
                0,
                system_program::id(),
                true,
                true,
                false,
                LAMPORTS_PER_SOL * 5,
            );
            let system_program = create_program_account(SYSTEM_PROGRAM_ID);

            let mut accounts = [counter, user, system_program];

            // let mut account_data = accounts
            //     .iter()
            //     .map(NativeAccountData::new_from_account_info)
            //     .collect::<Vec<_>>();
            let account_infos = accounts
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

// fn from_bin(data: &[u8]) -> SystemInstruction {
//     let data: SystemInstruction = deserialize(data).unwrap();
//     data
// }

struct TestSyscallStubs {}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_log(&self, message: &str) {
        println!("{message}");
    }
    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_log_compute_units(&self) {
        self.sol_log("SyscallStubs: sol_log_compute_units() not available");
    }
    fn sol_invoke_signed(
        &self,
        _instruction: &Instruction,
        _account_infos: &[AccountInfo],
        _signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        // create account within anchor lang
        // https://github.com/coral-xyz/anchor/blob/8f3bb8a5567047c7385adc25d3c8c0d7532aa01a/lang/src/system_program.rs#L158
        // leads to crate::solana_program::program::invoke_signed
        // https://github.com/solana-labs/solana/blob/a9509f56b7897b08bcdd16d3a056257a7d396681/sdk/program/src/program.rs#L247
        // invoke signed leads to invoke_signed_unchecked which then leads to crate::program_stubs::sol_invoke_signed
        // https://github.com/solana-labs/solana/blob/a9509f56b7897b08bcdd16d3a056257a7d396681/sdk/program/src/program.rs#L313

        // - There we have I think two options, one omit whole sol_invoke_signed stub logic, this means that invoke signed
        // within programs will lead into basically nothing
        // - We need to intercept instructions inside sol_invoke_signed find out what instruction it is, and call it by hand

        //https://github.com/solana-labs/solana/blob/a9509f56b7897b08bcdd16d3a056257a7d396681/programs/system/src/system_processor.rs#L317
        // In order to implement this we would neeed to skip whole
        // invoke signed logic and straight call system instructions so
        // create_account/assign/transfer etc. Which is probably not that easy
        // if _instruction.program_id == SYSTEM_PROGRAM_ID {
        //     let inst = from_bin(&_instruction.data);
        //     match inst {
        //         SystemInstruction::CreateAccount {
        //             lamports,
        //             space,
        //             owner,
        //         } => {
        //             solana_system_program::system_processor::
        //         }
        //         SystemInstruction::Assign { owner } => todo!(),
        //         SystemInstruction::Transfer { lamports } => todo!(),
        //         SystemInstruction::CreateAccountWithSeed {
        //             base,
        //             seed,
        //             lamports,
        //             space,
        //             owner,
        //         } => todo!(),
        //         SystemInstruction::AdvanceNonceAccount => todo!(),
        //         SystemInstruction::WithdrawNonceAccount(_) => todo!(),
        //         SystemInstruction::InitializeNonceAccount(_) => todo!(),
        //         SystemInstruction::AuthorizeNonceAccount(_) => todo!(),
        //         SystemInstruction::Allocate { space } => todo!(),
        //         SystemInstruction::AllocateWithSeed {
        //             base,
        //             seed,
        //             space,
        //             owner,
        //         } => todo!(),
        //         SystemInstruction::AssignWithSeed { base, seed, owner } => todo!(),
        //         SystemInstruction::TransferWithSeed {
        //             lamports,
        //             from_seed,
        //             from_owner,
        //         } => todo!(),
        //         SystemInstruction::UpgradeNonceAccount => todo!(),
        //     }
        // }
        self.sol_log("SyscallStubs: sol_invoke_signed() not available");
        Ok(())
    }
    // For now do not include these as we do not know what behavior we should expect
    // if we implement them this way
    // fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
    //     0
    // }
    // fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
    //     0
    // }
    // fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
    //     0
    // }
    // fn sol_get_return_data(&self) -> Option<(Pubkey, Vec<u8>)> {
    //     None
    // }
    // fn sol_set_return_data(&self, _data: &[u8]) {}
    // fn sol_get_processed_sibling_instruction(&self, _index: usize) -> Option<Instruction> {
    //     None
    // }
    // fn sol_get_stack_height(&self) -> u64 {
    //     0
    // }
}

fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
    });
}
