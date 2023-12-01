use crate::config::Config;
use crate::{
    idl::{self, Idl},
    Client,
};
use cargo_metadata::{MetadataCommand, Package};
use fehler::{throw, throws};
use futures::future::try_join_all;
use log::debug;
use solana_sdk::signer::keypair::Keypair;
use std::{io, os::unix::process::CommandExt, path::Path, process::Stdio, string::FromUtf8Error};
use thiserror::Error;
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::{Child, Command},
    signal,
};

use crate::constants::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    Io(#[from] io::Error),
    #[error("{0:?}")]
    Utf8(#[from] FromUtf8Error),
    #[error("localnet is not running")]
    LocalnetIsNotRunning,
    #[error("localnet is still running")]
    LocalnetIsStillRunning,
    #[error("build programs failed")]
    BuildProgramsFailed,
    #[error("testing failed")]
    TestingFailed,
    #[error("read program code failed: '{0}'")]
    ReadProgramCodeFailed(String),
    #[error("{0:?}")]
    Idl(#[from] idl::Error),
    #[error("{0:?}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("parsing Cargo.toml dependencies failed")]
    ParsingCargoTomlDependenciesFailed,
    #[error("fuzzing failed")]
    FuzzingFailed,
    #[error("Trdelnik it not correctly initialized! The trdelnik-tests folder in the root of your project does not exist")]
    NotInitialized,
    #[error("the crash file does not exist")]
    CrashFileNotFound,
}

/// Localnet (the validator process) handle.
pub struct LocalnetHandle {
    solana_test_validator_process: Child,
}

impl LocalnetHandle {
    /// Stops the localnet.
    ///
    /// _Note_: Manual kill: `kill -9 $(lsof -t -i:8899)`
    ///
    /// # Errors
    ///
    /// It fails when:
    /// - killing the process failed.
    /// - process is still running after the kill command has been performed.
    #[throws]
    pub async fn stop(mut self) {
        self.solana_test_validator_process.kill().await?;
        if Client::new(Keypair::new()).is_localnet_running(false).await {
            throw!(Error::LocalnetIsStillRunning);
        }
        debug!("localnet stopped");
    }

    /// Stops the localnet and removes the ledger.
    ///
    /// _Note_: Manual kill: `kill -9 $(lsof -t -i:8899)`
    ///
    /// # Errors
    ///
    /// It fails when:
    /// - killing the process failed.
    /// - process is still running after the kill command has been performed.
    /// - cannot remove localnet data (the `test-ledger` directory).
    #[throws]
    pub async fn stop_and_remove_ledger(self) {
        self.stop().await?;
        fs::remove_dir_all("test-ledger").await?;
        debug!("ledger removed");
    }
}

/// `Commander` allows you to start localnet, build programs,
/// run tests and do other useful operations.
pub struct Commander {
    // root: Cow<'static, str>,
}

impl Commander {
    /// Builds programs (smart contracts).
    #[throws]
    pub async fn build_programs() {
        let exit = std::process::Command::new("cargo")
            .arg("build-bpf")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        if !exit.status.success() {
            std::process::exit(exit.status.code().unwrap_or(1));
        }
    }

    /// Runs standard Rust tests.
    ///
    /// _Note_: The [--nocapture](https://doc.rust-lang.org/cargo/commands/cargo-test.html#display-options) argument is used
    /// to allow you read `println` outputs in your terminal window.
    #[throws]
    pub async fn run_tests() {
        let success = Command::new("cargo")
            .arg("test")
            .arg("--")
            .arg("--nocapture")
            .spawn()?
            .wait()
            .await?
            .success();
        if !success {
            throw!(Error::TestingFailed);
        }
    }
    /// Runs fuzzer on the given target.
    #[throws]
    pub async fn run_fuzzer(target: String, root: &Path) {
        let config = Config::new();

        let hfuzz_run_args = std::env::var("HFUZZ_RUN_ARGS").unwrap_or_default();
        let fuzz_args = config.get_fuzz_args(hfuzz_run_args);

        let cur_dir = root.join(TESTS_WORKSPACE_DIRECTORY);
        if !cur_dir.try_exists()? {
            throw!(Error::NotInitialized);
        }

        let mut child = Command::new("cargo")
            .env("HFUZZ_RUN_ARGS", fuzz_args)
            .current_dir(cur_dir)
            .arg("hfuzz")
            .arg("run")
            .arg(target)
            .spawn()?;

        tokio::select! {
            res = child.wait() =>
                match res {
                    Ok(status) => if !status.success() {
                        println!("Honggfuzz exited with an error!");
                    },
                    Err(_) => throw!(Error::FuzzingFailed),
            },
            _ = signal::ctrl_c() => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            },
        }
    }

    /// Runs fuzzer on the given target.
    #[throws]
    pub async fn run_fuzzer_debug(target: String, crash_file_path: String, root: &Path) {
        let cur_dir = root.join(TESTS_WORKSPACE_DIRECTORY);
        let crash_file = std::env::current_dir()?.join(crash_file_path);

        if !cur_dir.try_exists()? {
            throw!(Error::NotInitialized);
        }

        if !crash_file.try_exists()? {
            println!("The crash file {:?} not found!", crash_file);
            throw!(Error::CrashFileNotFound);
        }

        // using exec rather than spawn and replacing current process to avoid unflushed terminal output after ctrl+c signal
        std::process::Command::new("cargo")
            .current_dir(cur_dir)
            .arg("hfuzz")
            .arg("run-debug")
            .arg(target)
            .arg(crash_file)
            .exec();

        eprintln!("cannot execute \"cargo hfuzz run-debug\" command");
    }

    /// Returns an [Iterator] of program [Package]s read from `Cargo.toml` files.
    pub fn program_packages() -> impl Iterator<Item = Package> {
        let cargo_toml_data = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Cargo.toml reading failed");

        cargo_toml_data.packages.into_iter().filter(|package| {
            // @TODO less error-prone test if the package is a _program_?
            // This will only consider Packages where path:
            // /home/xyz/xyz/trdelnik/trdelnik/examples/example_project/programs/package1
            // NOTE we can obtain more important information here, only to remember
            if let Some("programs") = package.manifest_path.iter().nth_back(2) {
                return true;
            }
            false
        })
    }

    /// Ontains required information from programs.
    #[throws]
    pub async fn obtain_program_idl(root: &Path) -> Option<Idl> {
        let idl_programs = Commander::program_packages().map(|package| async move {
            let absolute_root = fs::canonicalize(root).await?;

            let name = package.name;
            let output = std::process::Command::new("cargo")
                .arg("+nightly")
                .arg("rustc")
                .args(["--package", &name])
                .arg("--profile=check")
                .arg("--")
                .arg("-Zunpretty=expanded")
                .output()
                .unwrap();
            if output.status.success() {
                let code = String::from_utf8(output.stdout)?;
                let path = package
                    .manifest_path
                    .parent()
                    .unwrap()
                    .strip_prefix(&absolute_root)
                    .unwrap()
                    .as_std_path();
                Ok(idl::parse_to_idl_program(name, &code, path).await?)
            } else {
                let error_text = String::from_utf8(output.stderr)?;
                Err(Error::ReadProgramCodeFailed(error_text))
            }
        });
        Some(Idl {
            programs: try_join_all(idl_programs).await?,
        })
    }

    /// Formats program code.
    #[throws]
    pub async fn format_program_code(code: &str) -> String {
        let mut rustfmt = Command::new("rustfmt")
            .args(["--edition", "2018"])
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        if let Some(stdio) = &mut rustfmt.stdin {
            stdio.write_all(code.as_bytes()).await?;
        }
        let output = rustfmt.wait_with_output().await?;
        String::from_utf8(output.stdout)?
    }

    /// Starts the localnet (Solana validator).
    #[throws]
    pub async fn start_localnet(root: &Path) -> LocalnetHandle {
        let mut process = Command::new("solana-test-validator")
            .arg("-C")
            .arg([root.to_str().unwrap(), "config.yml"].concat())
            .arg("-r")
            .arg("-q")
            .spawn()?;

        if !Client::new(Keypair::new()).is_localnet_running(true).await {
            // The validator might not be running, but the process might be still alive (very slow start, some bug, ...),
            // therefore we want to kill it if it's still running so ports aren't held.
            process.kill().await.ok();
            throw!(Error::LocalnetIsNotRunning);
        }
        debug!("localnet started");
        LocalnetHandle {
            solana_test_validator_process: process,
        }
    }

    /// Returns `use` modules / statements
    /// The goal of this method is to find all `use` statements defined by the user in the `.program_client`
    /// crate. It solves the problem with regenerating the program client and removing imports defined by
    /// the user.
    // TODO is this relevant when program_client should not be changed by user ?
    #[throws]
    pub async fn parse_program_client_imports() -> Option<Vec<syn::ItemUse>> {
        let output = std::process::Command::new("cargo")
            .arg("+nightly")
            .arg("rustc")
            .args(["--package", "program_client"])
            .arg("--profile=check")
            .arg("--")
            .arg("-Zunpretty=expanded")
            .output()
            .unwrap();

        if output.status.success() {
            let code = String::from_utf8(output.stdout)?;
            let mut use_modules: Vec<syn::ItemUse> = vec![];
            for item in syn::parse_file(code.as_str()).unwrap().items.into_iter() {
                if let syn::Item::Mod(module) = item {
                    let modules = module
                        .content
                        .ok_or("account mod: empty content")
                        .unwrap()
                        .1
                        .into_iter();
                    for module in modules {
                        if let syn::Item::Use(u) = module {
                            use_modules.push(u);
                        }
                    }
                }
            }
            if use_modules.is_empty() {
                use_modules.push(syn::parse_quote! { use trdelnik_client::*; })
            }
            Some(use_modules)
        } else {
            let mut use_modules: Vec<syn::ItemUse> = vec![];
            if use_modules.is_empty() {
                use_modules.push(syn::parse_quote! { use trdelnik_client::*; })
            }
            Some(use_modules)
        }
    }

    #[throws]
    pub async fn clean_anchor_target() {
        Command::new("anchor").arg("clean").spawn()?.wait().await?;
    }
    #[throws]
    pub async fn clean_hfuzz_target(root: &Path) {
        let hfuzz_target_path = root.join(TESTS_WORKSPACE_DIRECTORY).join(HFUZZ_TARGET);
        if hfuzz_target_path.exists() {
            fs::remove_dir_all(hfuzz_target_path).await?;
        } else {
            println!(
                "skipping {}/{} directory: not found",
                TESTS_WORKSPACE_DIRECTORY, HFUZZ_TARGET
            )
        }
    }
    #[throws]
    pub async fn run_fuzzer_with_exit_code(target: String, root: &Path) {
        // obtain config data
        let config = Config::new();
        // obtain hfuzz_run_args
        let hfuzz_run_args = std::env::var("HFUZZ_RUN_ARGS").unwrap_or_default();
        // obtain string from config and hfuzz_run_args
        let fuzz_args = config.get_fuzz_args(hfuzz_run_args);
        // Parse the fuzz_args arguments to find out if the crash folder and crash files extension was modified.
        // This will give precedence to latter
        let (crash_dir, ext) = get_crash_dir_and_ext(root, &target, &fuzz_args);

        if let Ok(crash_files) = get_crash_files(&crash_dir, &ext) {
            if !crash_files.is_empty() {
                println!("Error: The crash directory {} already contains crash files from previous runs. \n\nTo run Trdelnik fuzzer with exit code, you must either (backup and) remove the old crash files or alternatively change the crash folder using for example the --crashdir option and the HFUZZ_RUN_ARGS env variable such as:\nHFUZZ_RUN_ARGS=\"--crashdir ./new_crash_dir\"", crash_dir.to_string_lossy());
                std::process::exit(1);
            }
        }

        let cur_dir = root.join(TESTS_WORKSPACE_DIRECTORY);
        if !cur_dir.try_exists()? {
            throw!(Error::NotInitialized);
        }

        let mut child = Command::new("cargo")
            .env("HFUZZ_RUN_ARGS", fuzz_args)
            .current_dir(cur_dir)
            .arg("hfuzz")
            .arg("run")
            .arg(target)
            .spawn()?;

        tokio::select! {
            res = child.wait() =>
                match res {
                    Ok(status) => if !status.success() {
                        println!("Honggfuzz exited with an error!");
                    },
                    Err(_) => throw!(Error::FuzzingFailed),
            },
            _ = signal::ctrl_c() => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            },
        }

        if let Ok(crash_files) = get_crash_files(&crash_dir, &ext) {
            if !crash_files.is_empty() {
                println!(
                    "The crash directory {} contains new fuzz test crashes. Exiting!",
                    crash_dir.to_string_lossy()
                );
                std::process::exit(1);
            }
        }
    }
}

fn get_crash_dir_and_ext(
    root: &Path,
    target: &str,
    hfuzz_run_args: &str,
) -> (std::path::PathBuf, String) {
    // FIXME: we split by whitespace without respecting escaping or quotes - same approach as honggfuzz-rs so there is no point to fix it here before the upstream is fixed
    let hfuzz_run_args = hfuzz_run_args.split_whitespace();

    let extension =
        get_cmd_option_value(hfuzz_run_args.clone(), "-e", "--ext").unwrap_or("fuzz".to_string());

    let crash_dir = get_cmd_option_value(hfuzz_run_args.clone(), "", "--cr")
        .or_else(|| get_cmd_option_value(hfuzz_run_args.clone(), "-W", "--w"));

    let crash_path = if let Some(dir) = crash_dir {
        Path::new(root).join(TESTS_WORKSPACE_DIRECTORY).join(dir)
    } else {
        Path::new(root)
            .join(TESTS_WORKSPACE_DIRECTORY)
            .join(HFUZZ_WORKSPACE)
            .join(target)
    };

    (crash_path, extension)
}

fn get_crash_files(
    dir: &std::path::PathBuf,
    extension: &str,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let paths = std::fs::read_dir(dir)?
        // Filter out all those directory entries which couldn't be read
        .filter_map(|res| res.ok())
        // Map the directory entries to paths
        .map(|dir_entry| dir_entry.path())
        // Filter out all paths with extensions other than `extension`
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == extension) {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    Ok(paths)
}

fn get_cmd_option_value<'a>(
    hfuzz_run_args: impl Iterator<Item = &'a str>,
    short_opt: &str,
    long_opt: &str,
) -> Option<String> {
    let mut args_iter = hfuzz_run_args;
    let mut value: Option<String> = None;

    // ensure short option starts with one dash and long option with two dashes
    let short_opt = format!("-{}", short_opt.trim_start_matches('-'));
    let long_opt = format!("--{}", long_opt.trim_start_matches('-'));

    while let Some(arg) = args_iter.next() {
        match arg.strip_prefix(&short_opt) {
            Some(val) if short_opt.len() > 1 => {
                if !val.is_empty() {
                    // -ecrash for crash extension with no space
                    value = Some(val.to_string());
                } else if let Some(next_arg) = args_iter.next() {
                    // -e crash for crash extension with space
                    value = Some(next_arg.to_string());
                } else {
                    value = None;
                }
            }
            _ => {
                if arg.starts_with(&long_opt) && long_opt.len() > 2 {
                    value = args_iter.next().map(|a| a.to_string());
                }
            }
        }
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cmd_options_parsing() {
        let mut command = String::from("-Q -v --extension fuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q --extension fuzz -v");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q -e fuzz -v");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q --extension fuzz -v --extension ");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, None);

        command = String::from("-Q --extension fuzz -v -e ");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, None);

        let mut command = String::from("--extension buzz -e fuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q -v -e fuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q -v -efuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q -v --ext fuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, Some("fuzz".to_string()));

        command = String::from("-Q -v --extfuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, None);

        command = String::from("-Q -v --workspace");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "--ext");
        assert_eq!(extension, None);

        command = String::from("-Q -v -e");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "", "--ext");
        assert_eq!(extension, None);

        command = String::from("-Q -v --ext fuzz");
        let args = command.split_whitespace();

        let extension = get_cmd_option_value(args, "-e", "");
        assert_eq!(extension, None);
    }

    #[test]
    fn test_get_crash_dir_and_ext() {
        let root = Path::new("/home/fuzz");
        let target = "target";
        let default_crash_path = Path::new(root)
            .join(TESTS_WORKSPACE_DIRECTORY)
            .join(HFUZZ_WORKSPACE)
            .join(target);

        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "");

        assert_eq!(crash_dir, default_crash_path);
        assert_eq!(&ext, "fuzz");

        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "-Q -e");

        assert_eq!(crash_dir, default_crash_path);
        assert_eq!(&ext, "fuzz");

        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "-Q -e crash");

        assert_eq!(crash_dir, default_crash_path);
        assert_eq!(&ext, "crash");

        // test absolute path
        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "-Q -W /home/crash -e crash");

        let expected_crash_path = Path::new("/home/crash");
        assert_eq!(crash_dir, expected_crash_path);
        assert_eq!(&ext, "crash");

        // test absolute path
        let (crash_dir, ext) =
            get_crash_dir_and_ext(root, target, "-Q --crash /home/crash -e crash");

        let expected_crash_path = Path::new("/home/crash");
        assert_eq!(crash_dir, expected_crash_path);
        assert_eq!(&ext, "crash");

        // test relative path
        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "-Q -W ../crash -e crash");

        let expected_crash_path = root.join(TESTS_WORKSPACE_DIRECTORY).join("../crash");
        assert_eq!(crash_dir, expected_crash_path);
        assert_eq!(&ext, "crash");

        // test relative path
        let (crash_dir, ext) = get_crash_dir_and_ext(root, target, "-Q --crash ../crash -e crash");

        let expected_crash_path = root.join(TESTS_WORKSPACE_DIRECTORY).join("../crash");
        assert_eq!(crash_dir, expected_crash_path);
        assert_eq!(&ext, "crash");

        // crash directory has precedence before workspace
        let (crash_dir, ext) =
            get_crash_dir_and_ext(root, target, "-Q --crash ../crash -W /workspace -e crash");

        let expected_crash_path = root.join(TESTS_WORKSPACE_DIRECTORY).join("../crash");
        assert_eq!(crash_dir, expected_crash_path);
        assert_eq!(&ext, "crash");
    }
}
