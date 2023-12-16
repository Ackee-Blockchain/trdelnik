use anyhow::{bail, Error};

use clap::Subcommand;
use fehler::throws;
use trdelnik_client::Commander;
use trdelnik_client::WorkspaceBuilder;

use crate::discover;

pub const TRDELNIK_TOML: &str = "Trdelnik.toml";

#[derive(Subcommand)]
#[allow(non_camel_case_types)]
pub enum FuzzCommand {
    /// Run fuzz target
    Run {
        /// Name of the fuzz target
        target: String,
        /// Trdelnik will return exit code 1 in case of found crash files in the crash folder. This is checked before and after the fuzz test run.
        #[arg(short, long)]
        with_exit_code: bool,
    },
    /// Debug fuzz target with crash file
    Run_Debug {
        /// Name of the fuzz target
        target: String,
        /// Path to the crash file
        crash_file_path: String,
    },
    // TODO make optional parameter target name that will create new fuzz test with specified name
    Add,
}

#[throws]
pub async fn fuzz(root: Option<String>, subcmd: FuzzCommand) {
    // FIXME is the root option necessarry ?
    let root = match root {
        Some(r) => r,
        _ => {
            let root = discover(TRDELNIK_TOML)?;
            if let Some(r) = root {
                r
            } else {
                bail!("It does not seem that Trdelnik is initialized because the Trdelnik.toml file was not found in any parent directory!");
            }
        }
    };

    match subcmd {
        FuzzCommand::Run {
            target,
            with_exit_code,
        } => {
            if with_exit_code {
                Commander::run_fuzzer_with_exit_code(target, root).await?;
            } else {
                Commander::run_fuzzer(target).await?;
            }
        }
        FuzzCommand::Run_Debug {
            target,
            crash_file_path,
        } => {
            Commander::run_fuzzer_debug(target, crash_file_path, root).await?;
        }
        FuzzCommand::Add => {
            let mut generator = WorkspaceBuilder::new_with_root(root);
            generator.add_fuzz_test().await?;
        }
    };
}
