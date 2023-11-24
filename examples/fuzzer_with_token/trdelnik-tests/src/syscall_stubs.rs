use solana_program::{
    account_info::AccountInfo,
    entrypoint::{self, ProgramResult},
    program_stubs,
};
use trdelnik_client::Instruction;

use crate::Clock;

pub fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
    });
}

struct TestSyscallStubs {}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    /// The Rent sysvar contains the rental rate.
    /// Currently, the rate is static and set in genesis.
    /// The Rent burn percentage is modified by manual
    /// feature activation.
    /// https://docs.solana.com/developing/runtime-facilities/sysvars#rent
    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        _signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        macro_rules! create_entry {
            ($process_instruction:path, $id_method:path) => {
                if instruction.program_id == $id_method() {
                    return $process_instruction(
                        &instruction.program_id,
                        account_infos,
                        &instruction.data,
                    );
                }
            };
        }
        create_entry!(spl_token::processor::Processor::process, spl_token::id);
        create_entry!(
            spl_token_lending::processor::process_instruction,
            spl_token_lending::id
        );
        Ok(())
    }
    /// Each slot has an estimated duration based on Proof of History.
    /// But in reality, slots may elapse faster and slower than this estimate.
    /// As a result, the Unix timestamp of a slot is generated based on oracle
    /// input from voting validators. This timestamp is calculated as the stake-weighted median
    /// of timestamp estimates provided by votes, bounded by the expected time
    /// elapsed since the start of the epoch.
    /// https://docs.solana.com/developing/runtime-facilities/sysvars#clock
    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let now = Clock::now();
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock::clone(&now);
            0
        }
    }
    /// https://docs.solana.com/developing/runtime-facilities/sysvars#epochschedule
    /// The EpochSchedule sysvar contains epoch scheduling constants
    /// that are set in genesis, and enables calculating the number of
    /// slots in a given epoch, the epoch for a given slot, etc.
    fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
}
