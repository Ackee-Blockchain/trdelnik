use solana_program::{account_info::AccountInfo, bpf_loader, clock::Epoch, pubkey::Pubkey};

#[derive(Clone)]
pub struct NativeAccountData {
    pub key: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub program_id: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
}

impl NativeAccountData {
    pub fn new(
        size: usize,
        program_id: Pubkey,
        is_mut: bool,
        is_sig: bool,
        is_exec: bool,
        lamp: u64,
    ) -> Self {
        Self {
            key: Pubkey::new_unique(),
            lamports: lamp,
            data: vec![0; size],
            program_id,
            is_signer: is_sig,
            is_writable: is_mut,
            executable: is_exec,
        }
    }

    pub fn new_from_account_info(account_info: &AccountInfo) -> Self {
        Self {
            key: *account_info.key,
            lamports: **account_info.lamports.borrow(),
            data: account_info.data.borrow().to_vec(),
            program_id: *account_info.owner,
            is_signer: account_info.is_signer,
            is_writable: account_info.is_writable,
            executable: account_info.executable,
        }
    }

    pub fn as_account_info(&mut self) -> AccountInfo {
        AccountInfo::new(
            &self.key,
            self.is_signer,
            self.is_writable,
            &mut self.lamports,
            &mut self.data[..],
            &self.program_id,
            self.executable,
            Epoch::default(),
        )
    }
}

pub fn create_program_account(program_id: Pubkey) -> NativeAccountData {
    let mut account_data = NativeAccountData::new(0, bpf_loader::id(), false, false, true, 0);
    account_data.key = program_id;
    account_data
}
