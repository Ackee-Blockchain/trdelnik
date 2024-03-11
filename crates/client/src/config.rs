use anyhow::Context;
use fehler::throw;
use serde::Deserialize;
use std::{collections::HashMap, env, fs, io, path::PathBuf};
use thiserror::Error;

use crate::constants::*;

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
pub struct Cfg {
    pub cfg_identifier: String,
    pub val: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Fuzz {
    pub rust_flags: Vec<Cfg>,
}
#[derive(Default, Debug, Deserialize, Clone)]
struct _Fuzz {
    #[serde(default)]
    pub allow_duplicate_txs: Option<bool>,
    #[serde(default)]
    pub fuzzing_with_stats: Option<bool>,
}
impl From<_Fuzz> for Fuzz {
    fn from(_f: _Fuzz) -> Self {
        let mut _self = Self { rust_flags: vec![] };

        // allow_duplicate_txs
        let allow_duplicate_txs = _f.allow_duplicate_txs.unwrap_or(false);

        _self.rust_flags.push(Cfg {
            cfg_identifier: "allow_duplicate_txs".to_string(),
            val: allow_duplicate_txs,
        });

        // fuzzing_with_stats
        let fuzzing_with_stats = _f.fuzzing_with_stats.unwrap_or(false);

        _self.rust_flags.push(Cfg {
            cfg_identifier: "fuzzing_with_stats".to_string(),
            val: fuzzing_with_stats,
        });

        _self
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct HonggFuzzArg {
    pub short_opt: Option<String>,
    pub long_opt: Option<String>,
    pub val: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct HonggFuzz {
    pub fuzz_args: Vec<HonggFuzzArg>,
    pub env_variables: HashMap<String, String>,
}
#[derive(Default, Debug, Deserialize, Clone)]
struct _HonggFuzz {
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
    /// Target compilation directory, defaults to "trident-tests/fuzz_tests/fuzzing/hfuzz_target" to not clash with cargo build's default target directory.
    /// CARGO_TARGET_DIR env variable
    pub cargo_target_dir: Option<String>,
    #[serde(default)]
    /// Honggfuzz working directory, defaults to "trident-tests/fuzz_tests/fuzzing/hfuzz_workspace".
    /// HFUZZ_WORKSPACE env variable
    pub hfuzz_workspace: Option<String>,
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

impl From<_HonggFuzz> for HonggFuzz {
    fn from(_f: _HonggFuzz) -> Self {
        let mut _self = Self {
            fuzz_args: vec![],
            env_variables: HashMap::default(),
        };

        // timeout
        let timeout = _f.timeout.unwrap_or(10);
        _self
            .fuzz_args
            .push(HonggFuzzArg::new("-t", "--timeout", &timeout.to_string()));

        // iterations
        let iterations = _f.iterations.unwrap_or(0);
        _self.fuzz_args.push(HonggFuzzArg::new(
            "-N",
            "--iterations",
            &iterations.to_string(),
        ));

        // threads
        let threads = _f.threads.unwrap_or(0);
        if threads > 0 {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("-n", "--threads", &threads.to_string()));
        }

        // keep_output
        let keep_output = _f.keep_output.unwrap_or(false);
        if keep_output {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("-Q", "--keep_output", ""));
        }
        // verbose
        let verbose = _f.verbose.unwrap_or(false);
        if verbose {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("-v", "--verbose", ""));
        }

        // exit_upon_crash
        let exit_upon_crash = _f.exit_upon_crash.unwrap_or(false);
        if exit_upon_crash {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("", "--exit_upon_crash", ""));
        }
        // mutations_per_run
        let mutations_per_run = _f.mutations_per_run.unwrap_or(6);
        _self.fuzz_args.push(HonggFuzzArg::new(
            "-r",
            "--mutations_per_run",
            &mutations_per_run.to_string(),
        ));
        // cargo_target_dir
        let cargo_target_dir = _f.cargo_target_dir.unwrap_or_default();
        if !cargo_target_dir.is_empty() {
            _self
                .env_variables
                .insert(CARGO_TARGET_DIR_ENV.to_owned(), cargo_target_dir);
        } else {
            _self.env_variables.insert(
                CARGO_TARGET_DIR_ENV.to_owned(),
                CARGO_TARGET_DIR_DEFAULT.to_owned(),
            );
        }
        // hfuzz_workspace
        let hfuzz_workspace = _f.hfuzz_workspace.unwrap_or_default();
        if !hfuzz_workspace.is_empty() {
            _self
                .env_variables
                .insert(HFUZZ_WORKSPACE_ENV.to_owned(), hfuzz_workspace);
        } else {
            _self.env_variables.insert(
                HFUZZ_WORKSPACE_ENV.to_owned(),
                HFUZZ_WORKSPACE_DEFAULT.to_owned(),
            );
        }
        // crashdir
        let crash_dir = _f.crashdir.unwrap_or_default();
        if !crash_dir.is_empty() {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("", "--crashdir", &crash_dir));
        }
        // extension
        let extension = _f.extension.unwrap_or_default();
        if !extension.is_empty() {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("-e", "--extension", &extension));
        }
        // run_time
        let run_time = _f.run_time.unwrap_or(0);
        _self
            .fuzz_args
            .push(HonggFuzzArg::new("", "--run_time", &run_time.to_string()));

        // max_file_size
        let max_file_size = _f.max_file_size.unwrap_or(1_048_576);
        _self.fuzz_args.push(HonggFuzzArg::new(
            "-F",
            "--max_file_size",
            &max_file_size.to_string(),
        ));
        // save_all
        let save_all = _f.save_all.unwrap_or(false);
        if save_all {
            _self
                .fuzz_args
                .push(HonggFuzzArg::new("-u", "--save_all", ""));
        }
        _self
    }
}

impl HonggFuzzArg {
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
    pub honggfuzz: HonggFuzz,
    pub fuzz: Fuzz,
}

#[derive(Default, Debug, Deserialize, Clone)]
struct _Config {
    #[serde(default)]
    pub test: Option<_Test>,
    #[serde(default)]
    pub honggfuzz: Option<_HonggFuzz>,
    #[serde(default)]
    pub fuzz: Option<_Fuzz>,
}

impl From<_Config> for Config {
    fn from(_c: _Config) -> Self {
        Self {
            test: _c.test.unwrap_or_default().into(),
            honggfuzz: _c.honggfuzz.unwrap_or_default().into(),
            fuzz: _c.fuzz.unwrap_or_default().into(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let root = Config::discover_root().expect("failed to find the root folder");
        let s = fs::read_to_string(root.join(TRIDENT_TOML).as_path())
            .expect("failed to read the Trident config file");
        let _config: _Config = toml::from_str(&s).expect("failed to parse the Trident config file");
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
    pub fn get_honggfuzz_args(&self, cli_input: String) -> String {
        // Tested on a few examples, HFUZZ_RUN_ARGS give precedence to the later arguments.
        // so if HFUZZ_RUN_ARGS="-t 10 -t 15" -> timeout 15s is applied.
        // That means we do not need to parse the arguments from the CLI;
        // thus, we can simply append them at the end, and the CLI will have precedence.

        let mut args: Vec<String> = self
            .honggfuzz
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
    pub fn get_rustflags_args(&self, cli_input: String) -> String {
        let mut args: Vec<String> = self
            .fuzz
            .rust_flags
            .iter()
            .map(|arg| {
                if arg.val {
                    format!("--cfg {}", arg.cfg_identifier)
                } else {
                    "".to_string()
                }
            })
            .collect();
        args.push(cli_input);
        args.join(" ")
    }
    pub fn get_env_arg(&self, env_variable: &str) -> String {
        let expect = format!("{env_variable} not found");
        self.honggfuzz
            .env_variables
            .get(env_variable)
            .expect(&expect)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    impl Default for HonggFuzz {
        fn default() -> Self {
            let mut env_variables: HashMap<String, String> = HashMap::default();
            env_variables.insert(
                HFUZZ_WORKSPACE_ENV.to_owned(),
                HFUZZ_WORKSPACE_DEFAULT.to_owned(),
            );
            env_variables.insert(
                CARGO_TARGET_DIR_ENV.to_owned(),
                CARGO_TARGET_DIR_DEFAULT.to_owned(),
            );
            Self {
                fuzz_args: vec![
                    HonggFuzzArg::new("-t", "--timeout", &10.to_string()),
                    HonggFuzzArg::new("-N", "--iterations", &0.to_string()),
                    HonggFuzzArg::new("-r", "--mutations_per_run", &6.to_string()),
                    HonggFuzzArg::new("-e", "--extension", "fuzz"),
                    HonggFuzzArg::new("", "--run_time", &0.to_string()),
                    HonggFuzzArg::new("-F", "--max_file_size", &1_048_576.to_string()),
                ],
                env_variables,
            }
        }
    }

    impl Default for Fuzz {
        fn default() -> Self {
            let rust_flags = vec![
                Cfg {
                    cfg_identifier: "allow_duplicate_txs".to_string(),
                    val: false,
                },
                Cfg {
                    cfg_identifier: "fuzzing_with_stats".to_string(),
                    val: false,
                },
            ];

            Self { rust_flags }
        }
    }

    use super::*;
    #[test]
    fn test_merge_and_precedence1() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_honggfuzz_args(String::default());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 "
        );
    }
    #[test]
    fn test_merge_and_precedence2() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_honggfuzz_args("-t 0 -N10 --exit_upon_crash".to_string());

        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 0 -N10 --exit_upon_crash"
        );
    }
    #[test]
    fn test_merge_and_precedence3() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };
        let env_var_string =
            config.get_honggfuzz_args("-t 100 -N 5000 -Q -v --exit_upon_crash".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 100 -N 5000 -Q -v --exit_upon_crash"
        );
    }
    #[test]
    fn test_merge_and_precedence4() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_honggfuzz_args("-t 10 -N 500 -Q -v --exit_upon_crash -n 15 --mutations_per_run 8 --verifier -W random_dir --crashdir random_dir5 --run_time 666".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 10 -N 500 -Q -v --exit_upon_crash -n 15 --mutations_per_run 8 --verifier -W random_dir --crashdir random_dir5 --run_time 666"
        );
    }
    #[test]
    fn test_merge_and_precedence5() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let env_var_string = config.get_honggfuzz_args("-t 10 -N 500 -Q -v --exit_upon_crash -n 15 --verifier -W random_dir --crashdir random_dir5 --run_time 666".to_string());
        assert_eq!(
            env_var_string,
            "-t 10 -N 0 -r 6 -e fuzz --run_time 0 -F 1048576 -t 10 -N 500 -Q -v --exit_upon_crash -n 15 --verifier -W random_dir --crashdir random_dir5 --run_time 666"
        );
    }
    #[test]
    fn test_obtain_env_variables() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let cargo_target_dir = config.get_env_arg(CARGO_TARGET_DIR_ENV);

        assert_eq!(cargo_target_dir, CARGO_TARGET_DIR_DEFAULT);
        let hfuzz_workspace = config.get_env_arg(HFUZZ_WORKSPACE_ENV);
        assert_eq!(hfuzz_workspace, HFUZZ_WORKSPACE_DEFAULT);
    }
    #[test]
    fn test_obtain_env_variables2() {
        let mut config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        config
            .honggfuzz
            .env_variables
            .insert(CARGO_TARGET_DIR_ENV.to_owned(), "new_value_x".to_owned());

        config
            .honggfuzz
            .env_variables
            .insert(HFUZZ_WORKSPACE_ENV.to_owned(), "new_value_y".to_owned());

        let cargo_target_dir = config.get_env_arg(CARGO_TARGET_DIR_ENV);

        assert_eq!(cargo_target_dir, "new_value_x");
        let hfuzz_workspace = config.get_env_arg(HFUZZ_WORKSPACE_ENV);
        assert_eq!(hfuzz_workspace, "new_value_y");
    }

    #[test]
    fn test_obtain_rustflags_variable1() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz::default(),
        };

        let rustflags = config.get_rustflags_args("".to_string());
        let default_rustflags = "  ";

        assert_eq!(rustflags, default_rustflags);
    }
    #[test]
    fn test_obtain_rustflags_variable2() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz {
                rust_flags: vec![Cfg {
                    cfg_identifier: "fuzzing_with_stats".to_string(),
                    val: true,
                }],
            },
        };

        let rustflags = config.get_rustflags_args("".to_string());
        let reference_rustflags = "--cfg fuzzing_with_stats ";

        assert_eq!(rustflags, reference_rustflags);
    }
    #[test]
    fn test_obtain_rustflags_variable3() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz {
                rust_flags: vec![
                    Cfg {
                        cfg_identifier: "allow_duplicate_txs".to_string(),
                        val: true,
                    },
                    Cfg {
                        cfg_identifier: "fuzzing_with_stats".to_string(),
                        val: false,
                    },
                ],
            },
        };

        let rustflags = config.get_rustflags_args("".to_string());
        let reference_rustflags = "--cfg allow_duplicate_txs  ";

        assert_eq!(rustflags, reference_rustflags);
    }
    #[test]
    fn test_obtain_rustflags_variable4() {
        let config = Config {
            test: Test::default(),
            honggfuzz: HonggFuzz::default(),
            fuzz: Fuzz {
                rust_flags: vec![Cfg {
                    cfg_identifier: "allow_duplicate_txs".to_string(),
                    val: true,
                }],
            },
        };

        let rustflags = config.get_rustflags_args("--cfg fuzzing_with_stats".to_string());
        let reference_rustflags = "--cfg allow_duplicate_txs --cfg fuzzing_with_stats";

        assert_eq!(rustflags, reference_rustflags);
    }
}
