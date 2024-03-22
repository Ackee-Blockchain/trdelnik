//! Trident is a suite of tools and libraries for testing, auditing and developing
//! [Solana](https://solana.com/) / [Anchor](https://book.anchor-lang.com/chapter_1/what_is_anchor.html) programs (smart contracts).
//!
//! Trident could be useful for writing Rust dApps, too.

pub use anchor_client::{
    self,
    anchor_lang::{self, prelude::System, Id, InstructionData, ToAccountMetas},
    solana_sdk::{
        self,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::Signature,
        signer::{keypair::Keypair, Signer},
    },
    ClientError,
};
pub use anyhow::{self, Error};

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    pub use self::anchor_lang::solana_program::instruction::AccountMeta;
    pub use super::{
        anchor_lang, anchor_lang::system_program::ID as SYSTEM_PROGRAM_ID,
        anchor_lang::InstructionData, anchor_lang::ToAccountInfo, anchor_lang::ToAccountMetas,
        fuzz_trident, show_account, solana_sdk::account::Account,
        solana_sdk::transaction::Transaction, Instruction, Keypair, Pubkey, Signer, TempClone,
    };
    pub use anchor_client::anchor_lang::solana_program::account_info::AccountInfo;
    pub use anchor_client::anchor_lang::solana_program::hash::Hash;
    pub use anchor_lang::Key;
    pub use arbitrary;
    pub use arbitrary::Arbitrary;
    pub use honggfuzz::fuzz;
    // TODO add optional feature gated dependency
    pub use solana_program_test_anchor_fix::{
        processor, tokio::runtime::Runtime, BanksClient, BanksClientError, ProgramTest,
        ProgramTestContext,
    };

    pub use super::error::*;
    pub use super::fuzzer::accounts_storage::*;
    pub use super::fuzzer::data_builder::build_ix_fuzz_data;
    pub use super::fuzzer::data_builder::*;
    pub use super::fuzzing_stats::FuzzingStatistics;

    pub use super::fuzzer::program_test_client_blocking::ProgramTestClientBlocking;
    pub use super::fuzzer::snapshot::Snapshot;
    pub use super::fuzzer::*;
    pub use std::cell::RefCell;
    pub use std::collections::HashMap;
    pub use trident_derive_displayix::DisplayIx;
    pub use trident_derive_fuzz_deserialize::FuzzDeserialize;
    pub use trident_derive_fuzz_test_executor::FuzzTestExecutor;
}

pub use futures::{self, FutureExt};
pub use rstest::*;
pub use serial_test;
pub use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
pub use tokio;

pub use trident_test::trident_test;

mod config;

mod client;
pub use client::Client;
pub use client::PrintableTransaction;

mod reader;
pub use reader::Reader;

mod commander;
pub use commander::{Commander, LocalnetHandle};

mod tester;
pub use tester::Tester;

mod temp_clone;
pub use temp_clone::TempClone;

mod keys;
pub use keys::*;

mod fuzzer;
pub use fuzzer::*;
pub mod idl;
pub mod program_client_generator;

pub mod test_generator;
pub use test_generator::TestGenerator;

pub mod error_reporter;
pub use error_reporter::*;

pub mod cleaner;
pub use cleaner::*;

// This is a workaround for tests: https://github.com/Ackee-Blockchain/trident/pull/112#issuecomment-1924920952
pub use trident_derive_displayix::DisplayIx;
pub use trident_derive_fuzz_deserialize::FuzzDeserialize;
pub use trident_derive_fuzz_test_executor::FuzzTestExecutor;

mod constants {
    // program_client
    pub const PROGRAM_CLIENT_DIRECTORY: &str = ".program_client";
    pub const LIB: &str = "lib.rs";
    pub const SRC_DIRECTORY: &str = "src";

    // tomls
    pub const CARGO_TOML: &str = "Cargo.toml";
    pub const TRIDENT_TOML: &str = "Trident.toml";
    pub const ANCHOR_TOML: &str = "Anchor.toml";

    // tests
    pub const TESTS_WORKSPACE_DIRECTORY: &str = "trident-tests";

    // poc
    pub const POC_TEST_DIRECTORY: &str = "poc_tests";
    pub const TESTS_DIRECTORY: &str = "tests";
    pub const POC_TEST: &str = "test.rs";

    // fuzz
    pub const ACCOUNTS_SNAPSHOTS_FILE_NAME: &str = "accounts_snapshots.rs";
    pub const FUZZ_INSTRUCTIONS_FILE_NAME: &str = "fuzz_instructions.rs";
    pub const FUZZ_TEST_DIRECTORY: &str = "fuzz_tests";
    pub const FUZZING: &str = "fuzzing";
    pub const FUZZ_TEST: &str = "test_fuzz.rs";
    pub const HFUZZ_TARGET: &str = "hfuzz_target";
    pub const CARGO_TARGET_DIR_DEFAULT: &str = "trident-tests/fuzz_tests/fuzzing/hfuzz_target";
    pub const HFUZZ_WORKSPACE_DEFAULT: &str = "trident-tests/fuzz_tests/fuzzing/hfuzz_workspace";
    pub const CARGO_TARGET_DIR_ENV: &str = "CARGO_TARGET_DIR";
    pub const HFUZZ_WORKSPACE_ENV: &str = "HFUZZ_WORKSPACE";

    // workspace
    pub const GIT_IGNORE: &str = ".gitignore";

    // client
    pub const RETRY_LOCALNET_EVERY_MILLIS: u64 = 500;
    pub const DEFAULT_KEYPAIR_PATH: &str = "~/.config/solana/id.json";

    // Formatting
    pub const SKIP: &str = "\x1b[33mSkip\x1b[0m";
    pub const WARNING: &str = "\x1b[1;93mWarning\x1b[0m";
    pub const FINISH: &str = "\x1b[92mFinished\x1b[0m";
    pub const ERROR: &str = "\x1b[31mError\x1b[0m";

    // special message for the progress bar
    pub const EXPANDING_PROGRESS_BAR: &str = "\x1b[92mExpanding\x1b[0m";
}
