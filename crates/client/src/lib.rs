//! Trdelnik is a suite of tools and libraries for testing, auditing and developing
//! [Solana](https://solana.com/) / [Anchor](https://book.anchor-lang.com/chapter_1/what_is_anchor.html) programs (smart contracts).
//!
//! Trdelnik could be useful for writing Rust dApps, too.

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
    pub use crate::native_account::*;
    pub use crate::native_clock::*;

    pub use crate::anchor_lang::{AccountDeserialize, AccountSerialize, InstructionData};
    pub use arbitrary;
    pub use arbitrary::Arbitrary;
    pub use honggfuzz::fuzz;
    pub use solana_program::{
        account_info::AccountInfo,
        entrypoint::{self, ProgramResult},
        native_token::LAMPORTS_PER_SOL,
        program_pack::*,
        program_stubs, system_program,
    };
    pub use solana_sdk::instruction::Instruction;
}

pub use futures::{self, FutureExt};
pub use rstest::*;
pub use serial_test;
pub use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
pub use tokio;

pub use trdelnik_test::trdelnik_test;

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

pub mod idl;
pub mod program_client_generator;

pub mod workspace_builder;
pub use workspace_builder::WorkspaceBuilder;

pub mod error_reporter;
pub use error_reporter::*;

pub mod native_clock;
pub use native_clock::*;

pub mod native_account;
pub use native_account::*;

pub mod constants {
    use std::collections::HashMap;

    pub const PROGRAM_CLIENT_DIRECTORY: &str = ".program_client";
    pub const CARGO: &str = "Cargo.toml";
    pub const LIB: &str = "lib.rs";
    pub const SRC: &str = "src";

    pub const TESTS_WORKSPACE_DIRECTORY: &str = "trdelnik-tests";
    pub const TEST_DIRECTORY: &str = "tests";
    pub const TEST: &str = "test.rs";

    pub const FUZZ_DIRECTORY: &str = "src/bin";
    pub const FUZZ: &str = "fuzz_target.rs";
    pub const PROGRAM_STUBS: &str = "program_stubs.rs";

    pub const PROGRAM_STUBS_ENTRIES: &str = "// ### \"Entrypoints go above\" ###";
    pub const HFUZZ_TARGET: &str = "hfuzz_target";
    pub const HFUZZ_WORKSPACE: &str = "hfuzz_workspace";

    pub const GIT_IGNORE: &str = ".gitignore";

    pub const CLIENT_TOML_TEMPLATE: &str = "/src/templates/program_client/Cargo.toml.tmpl";

    lazy_static::lazy_static! {
        pub static ref PROCESS_INSTRUCTIONS: HashMap<&'static str, (&'static str, &'static str,&'static str,&'static str)> = HashMap::from([(
        "Token",
        (
            "spl_token::id",
            "spl_token::processor::Processor::process",
            "spl-token",
            "4.0.0"
        ),
    )]);
    }
}
