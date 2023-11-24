// DO NOT EDIT - automatically generated file (Edit only at your own risk !!!)

use trdelnik_client::fuzzing::*;

pub fn test_syscall_stubs() {
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
        macro_rules! create_processor {
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
        // ### "Entrypoints go above" ###
        Ok(())
    }
    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let now = Clock::now();
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock::clone(&now);
            0
        }
    }
    fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
}
