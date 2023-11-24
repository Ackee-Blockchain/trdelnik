use anyhow::Context;
use fehler::throw;
use serde::Deserialize;
use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

pub const CARGO_TOML: &str = "Cargo.toml";
pub const TRDELNIK_TOML: &str = "Trdelnik.toml";
pub const ANCHOR_TOML: &str = "Anchor.toml";

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid workspace")]
    BadWorkspace,
    #[error("{0:?}")]
    Anyhow(#[from] anyhow::Error),
    #[error("{0:?}")]
    Io(#[from] io::Error),
    #[error("{0:?}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Test {
    pub validator_startup_timeout: u64,
}
#[derive(Default, Debug, Deserialize, Clone)]
struct _Test {
    #[serde(default)]
    pub validator_startup_timeout: Option<u64>,
}
impl Default for Test {
    fn default() -> Self {
        Self {
            validator_startup_timeout: 10_000,
        }
    }
}
impl From<_Test> for Test {
    fn from(_t: _Test) -> Self {
        Self {
            validator_startup_timeout: _t.validator_startup_timeout.unwrap_or(10_000),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct FuzzArg {
    pub short_opt: Option<String>,
    pub long_opt: Option<String>,
    pub val: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct Fuzz {
    pub fuzz_args: Vec<FuzzArg>,
}
#[derive(Default, Debug, Deserialize, Clone)]
struct _Fuzz {
    #[serde(default)]
    /// Timeout in seconds (default: 10)
    /// -t
    /// --timeout
    pub timeout: Option<u16>,
    #[serde(default)]
    /// Number of fuzzing iterations (default: 0 [no limit])
    /// -N
    /// --iterations
    pub iterations: Option<u64>,
    #[serde(default)]
    /// Number of concurrent fuzzing threads (default: number of CPUs / 2)
    /// -n
    /// --threads
    pub threads: Option<u16>,
    #[serde(default)]
    /// Don't close children's stdin, stdout, stderr; can be noisy
    /// -Q
    /// --keep_output
    pub keep_output: Option<bool>,
    #[serde(default)]
    /// Disable ANSI console; use simple log output
    /// -v
    /// --verbose
    pub verbose: Option<bool>,
    #[serde(default)]
    /// Exit upon seeing the first crash (default: false)
    /// --exit_upon_crash
    pub exit_upon_crash: Option<bool>,
    #[serde(default)]
    /// Maximal number of mutations per one run (default: 6)
    /// -r
    /// --mutations_per_run
    pub mutations_per_run: Option<u16>,
    #[serde(default)]
    /// Directory where crashes are saved to (default: workspace directory)
    /// --crashdir
    pub crashdir: Option<String>,
    #[serde(default)]
    /// Input file extension (e.g. 'swf'), (default: 'fuzz')
    /// -e
    /// --extension
    pub extension: Option<String>,
    #[serde(default)]
    /// Number of seconds this fuzzing session will last (default: 0 [no limit])
    /// --run_time
    pub run_time: Option<u32>,
    #[serde(default)]
    /// Maximal size of files processed by the fuzzer in bytes (default: 1048576 = 1MB)
    /// -F
    /// --max_file_size
    pub max_file_size: Option<u32>,
    #[serde(default)]
    /// Save all test-cases (not only the unique ones) by appending the current time-stamp to the filenames (default: false)
    /// -u
    /// --save_all
    pub save_all: Option<bool>,
}
impl Default for Fuzz {
    fn default() -> Self {
        Self {
            fuzz_args: vec![
                FuzzArg::new("-t", "--timeout", &10.to_string()),
                FuzzArg::new("-N", "--iterations", &0.to_string()),
                FuzzArg::new("-r", "--mutations_per_run", &6.to_string()),
                FuzzArg::new("-e", "--extension", "fuzz"),
                FuzzArg::new("", "--run_time", &0.to_string()),
                FuzzArg::new("-F", "--max_file_size", &1_048_576.to_string()),
            ],
        }
    }
}
impl From<_Fuzz> for Fuzz {
    fn from(_f: _Fuzz) -> Self {
        let mut _self = Self { fuzz_args: vec![] };

        // timeout
        let timeout = _f.timeout.unwrap_or(10);
        _self
            .fuzz_args
            .push(FuzzArg::new("-t", "--timeout", &timeout.to_string()));

        // iterations
        let iterations = _f.iterations.unwrap_or(0);
        _self
            .fuzz_args
            .push(FuzzArg::new("-N", "--iterations", &iterations.to_string()));

        // threads
        let threads = _f.threads.unwrap_or(0);
        if threads > 0 {
            _self
                .fuzz_args
                .push(FuzzArg::new("-n", "--threads", &threads.to_string()));
        }

        // keep_output
        let keep_output = _f.keep_output.unwrap_or(false);
        if keep_output {
            _self
                .fuzz_args
                .push(FuzzArg::new("-Q", "--keep_output", ""));
        }

        // verbose
        let verbose = _f.verbose.unwrap_or(false);
        if verbose {
            _self.fuzz_args.push(FuzzArg::new("-v", "--verbose", ""));
        }

        // exit_upon_crash
        let exit_upon_crash = _f.exit_upon_crash.unwrap_or(false);
        if exit_upon_crash {
            _self
                .fuzz_args
                .push(FuzzArg::new("", "--exit_upon_crash", ""));
        }
        // mutations_per_run
        let mutations_per_run = _f.mutations_per_run.unwrap_or(6);
        _self.fuzz_args.push(FuzzArg::new(
            "-r",
            "--mutations_per_run",
            &mutations_per_run.to_string(),
        ));
        // crashdir
        let crash_dir = _f.crashdir.unwrap_or_default();
        if !crash_dir.is_empty() {
            _self
                .fuzz_args
                .push(FuzzArg::new("", "--crashdir", &crash_dir));
        }
        // extension
        let extension = _f.extension.unwrap_or_default();
        if !extension.is_empty() {
            _self
                .fuzz_args
                .push(FuzzArg::new("-e", "--extension", &extension));
        }
        // run_time
        let run_time = _f.run_time.unwrap_or(0);
        _self
            .fuzz_args
            .push(FuzzArg::new("", "--run_time", &run_time.to_string()));

        // max_file_size
        let max_file_size = _f.max_file_size.unwrap_or(1_048_576);
        _self.fuzz_args.push(FuzzArg::new(
            "-F",
            "--max_file_size",
            &max_file_size.to_string(),
        ));
        // save_all
        let save_all = _f.save_all.unwrap_or(false);
        if save_all {
            _self.fuzz_args.push(FuzzArg::new("-u", "--save_all", ""));
        }
        _self
    }
}

impl FuzzArg {
    fn new(short_opt: &str, long_opt: &str, val: &str) -> Self {
        let short_opt = if short_opt.is_empty() {
            None
        } else {
            Some(short_opt.to_owned())
        };
        let long_opt = if long_opt.is_empty() {
            None
        } else {
            Some(long_opt.to_owned())
        };
        let val = if val.is_empty() {
            None
        } else {
            Some(val.to_owned())
        };
        Self {
            short_opt,
            long_opt,
            val,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub test: Test,
    pub fuzz: Fuzz,
}

#[derive(Default, Debug, Deserialize, Clone)]
struct _Config {
    #[serde(default)]
    pub test: Option<_Test>,
    #[serde(default)]
    pub fuzz: Option<_Fuzz>,
}

impl From<_Config> for Config {
    fn from(_c: _Config) -> Self {
        Self {
            test: _c.test.unwrap_or_default().into(),
            fuzz: _c.fuzz.unwrap_or_default().into(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let root = Config::discover_root().expect("failed to find the root folder");
        let s = fs::read_to_string(root.join(TRDELNIK_TOML).as_path())
            .expect("failed to read the Trdelnik config file");
        let _config: _Config =
            toml::from_str(&s).expect("failed to parse the Trdelnik config file");
        _config.into()
    }

    /// Tries to find the root directory with the `Anchor.toml` file.
    /// Throws an error when there is no directory with the `Anchor.toml` file
    pub fn discover_root() -> Result<PathBuf, Error> {
        let current_dir = env::current_dir()?;
        let mut dir = Some(current_dir.as_path());
        while let Some(cwd) = dir {
            for file in std::fs::read_dir(cwd).with_context(|| {
                format!("Error reading the directory with path: {}", cwd.display())
            })? {
                let path = file
                    .with_context(|| {
                        format!("Error reading the directory with path: {}", cwd.display())
                    })?
                    .path();
                if let Some(filename) = path.file_name() {
                    if filename.to_str() == Some(ANCHOR_TOML) {
                        return Ok(PathBuf::from(cwd));
                    }
                }
            }
            dir = cwd.parent();
        }
        throw!(Error::BadWorkspace)
    }
    pub fn get_fuzz_args(self, cli_input: String) -> String {
        // Tested on a few examples, HFUZZ_RUN_ARGS give precedence to the later arguments.
        // so if HFUZZ_RUN_ARGS="-t 10 -t 15" -> timeout 15s is applied.
        // That means we do not need to parse the arguments from the CLI;
        // thus, we can simply append them at the end, and the CLI will have precedence.

        let mut args: Vec<String> = self
            .fuzz
            .fuzz_args
            .iter()
            .map(|a| {
                let val = a.val.to_owned().unwrap_or("".to_string());
                if let Some(o) = &a.short_opt {
                    format!("{} {}", o, val)
                } else if let Some(o) = &a.long_opt {
                    format!("{} {}", o, val)
                } else {
                    "".to_string()
                }
            })
            .collect();
        args.push(cli_input);
        args.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_merge_and_precedence1() {
        let config = Config {
            test: Test::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_fuzz_args(String::default());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 "
        );
    }
    #[test]
    fn test_merge_and_precedence2() {
        let config = Config {
            test: Test::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_fuzz_args("-t 0 -N10 --exit_upon_crash".to_string());

        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 0 -N10 --exit_upon_crash"
        );
    }
    #[test]
    fn test_merge_and_precedence3() {
        let config = Config {
            test: Test::default(),
            fuzz: Fuzz::default(),
        };
        let env_var_string =
            config.get_fuzz_args("-t 100 -N 5000 -Q -v --exit_upon_crash".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 100 -N 5000 -Q -v --exit_upon_crash"
        );
    }
    #[test]
    fn test_merge_and_precedence4() {
        let config = Config {
            test: Test::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_fuzz_args("-t 10 -N 500 -Q -v --exit_upon_crash -n 15 --mutations_per_run 8 --verifier -W random_dir --crashdir random_dir5 --run_time 666".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 10 -N 500 -Q -v --exit_upon_crash -n 15 --mutations_per_run 8 --verifier -W random_dir --crashdir random_dir5 --run_time 666"
        );
    }
    #[test]
    fn test_merge_and_precedence5() {
        let config = Config {
            test: Test::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_fuzz_args("-t 10 -N 500 -Q -v --exit_upon_crash -n 15 --verifier -W random_dir --crashdir random_dir5 --run_time 666".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 10 -N 500 -Q -v --exit_upon_crash -n 15 --verifier -W random_dir --crashdir random_dir5 --run_time 666"
        );
    }
}
