use crate::{
    commander::{Commander, Error as CommanderError},
    fuzzer,
    idl::IdlProgram,
    program_client_generator,
    snapshot_generator::generate_snapshots_code,
};
use cargo_metadata::{camino::Utf8PathBuf, Package};
use fehler::{throw, throws};
use std::{fs::File, io::prelude::*};
use std::{
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
};
use syn::ItemUse;
use thiserror::Error;
use tokio::fs;
use toml::{value::Table, Value};

use crate::constants::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot parse Cargo.toml")]
    CannotParseCargoToml,
    #[error("{0:?}")]
    Io(#[from] io::Error),
    #[error("{0:?}")]
    Toml(#[from] toml::de::Error),
    #[error("{0:?}")]
    Commander(#[from] CommanderError),
    #[error("Cannot find the Anchor.toml file to locate the root folder")]
    BadWorkspace,
    #[error("The Anchor project does not contain any programs")]
    NoProgramsFound,
    #[error("read program code failed: '{0}'")]
    ReadProgramCodeFailed(String),
    #[error("parsing Cargo.toml dependencies failed")]
    ParsingCargoTomlDependenciesFailed,
}

/// Constructs a `PathBuf` from a given root and a series of path components.
///
/// This macro simplifies the creation of a `PathBuf` from multiple strings or string slices,
/// starting with a root path and appending each subsequent component in order. It's useful for
/// dynamically constructing file or directory paths in a more readable manner.
///
/// # Syntax
/// `construct_path!(root, component1, component2, ..., componentN)`
///
/// - `root`: The base path from which to start. Can be a `PathBuf` or any type that implements
///   `Into<PathBuf>`, such as a string or string slice.
/// - `component1` to `componentN`: These are the components to be joined to the root path. Each
///   can be any type that implements `Into<PathBuf>`, allowing for flexible path construction.
///
/// # Returns
/// - Returns a `PathBuf` representing the combined path.
///
/// # Examples
/// Basic usage:
///
/// ```
/// # #[macro_use] extern crate trident_client;
/// # fn main() {
/// use std::path::PathBuf;
///
/// // Constructs a PathBuf from a series of string slices
/// let path = construct_path!(PathBuf::from("/tmp"), "my_project", "src", "main.rs");
/// assert_eq!(path, PathBuf::from("/tmp/my_project/src/main.rs"));
/// # }
/// ```
///
/// Note: Replace `your_crate_name` with the name of your crate where this macro is defined.

#[macro_export]
macro_rules! construct_path {
    ($root:expr, $($component:expr),*) => {
        {
            let mut path = $root.to_owned();
            $(path = path.join($component);)*
            path
        }
    };
}

/// Includes a file as a string at compile time.
macro_rules! load_template {
    ($file:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $file))
    };
}

#[derive(Clone)]
pub struct ProgramData {
    pub code: String,
    pub path: Utf8PathBuf,
    pub program_idl: IdlProgram,
}

/// Represents a generator for creating tests.
///
/// This struct is designed to hold all necessary information for generating
/// test cases for a project. It includes paths to project components,
/// interface definitions, and additional configuration for code generation.
///
/// # Fields
/// - `root`: A `PathBuf` indicating the root directory of the project for which tests are being generated.
/// This path is used as a base for any relative paths within the project.
/// - `programs_data`: A vector of tuples, each containing a `String`, `Utf8PathBuf` and IDL Program data.
/// Each tuple represents a pair of code and the package path associated with it.
/// - `packages`: A vector of `Package` structs, representing the different packages
/// that make up the project.
/// - `use_tokens`: A vector of `ItemUse` tokens from the Rust syntax, representing `use` statements that
/// should be included in the generated code for .program_client.
pub struct TestGenerator {
    pub root: PathBuf,
    pub programs_data: Vec<ProgramData>,
    pub packages: Vec<Package>,
    pub use_tokens: Vec<ItemUse>,
}
impl Default for TestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TestGenerator {
    /// Creates a new instance of `TestGenerator` with default values.
    ///
    /// # Returns
    ///
    /// A new `TestGenerator` instance.
    pub fn new() -> Self {
        Self {
            root: Path::new("../../").to_path_buf(),
            programs_data: Vec::default(),
            packages: Vec::default(),
            use_tokens: Vec::default(),
        }
    }
    /// Creates a new instance of `TestGenerator` with a specified root directory.
    ///
    /// # Arguments
    ///
    /// * `root` - A string slice that holds the path to the root directory.
    ///
    /// # Returns
    ///
    /// A new `TestGenerator` instance with the specified root directory.
    pub fn new_with_root(root: String) -> Self {
        Self {
            root: Path::new(&root).to_path_buf(),
            programs_data: Vec::default(),
            packages: Vec::default(),
            use_tokens: Vec::default(),
        }
    }
    /// Generates both proof of concept (POC) and fuzz tests along with the necessary setup.
    #[throws]
    pub async fn generate_both(&mut self) {
        let root_path = self.root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        // build first
        commander.build_anchor_project().await?;
        // expands programs within programs folder
        self.expand_programs().await?;
        // obtain use statements
        // if program_client is not yet initialized
        // use statements are set to default
        self.get_program_client_imports().await?;
        // initialize program client if it is not yet initialized
        self.init_program_client().await?;
        // initialize poc tests if thay are not yet initialized
        self.init_poc_tests().await?;
        // initialize fuzz tests if thay are not yet initialized
        self.init_fuzz_tests().await?;
        // add trident.toml
        self.create_trident_manifest().await?;
        // update gitignore
        self.update_gitignore(CARGO_TARGET_DIR_DEFAULT)?;
    }

    /// Generates fuzz tests along with the necessary setup.
    #[throws]
    pub async fn generate_fuzz(&mut self) {
        let root_path = self.root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        // build first
        commander.build_anchor_project().await?;
        // expand programs
        self.expand_programs().await?;
        // initialize fuzz tests if thay are not yet initialized
        self.init_fuzz_tests().await?;
        // add trident.toml
        self.create_trident_manifest().await?;
        // update gitignore
        self.update_gitignore(CARGO_TARGET_DIR_DEFAULT)?;
    }
    /// Generates proof of concept (POC) tests along with the necessary setup.
    #[throws]
    pub async fn generate_poc(&mut self) {
        let root_path = self.root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        // build first
        commander.build_anchor_project().await?;
        // expand programs
        self.expand_programs().await?;
        // obtain use statements
        // if program_client is not yet initialized
        // use statements are set to default
        self.get_program_client_imports().await?;
        // initialize program client if it is not yet initialized
        self.init_program_client().await?;
        // initialize poc tests if thay are not yet initialized
        self.init_poc_tests().await?;
        // add trident.toml
        self.create_trident_manifest().await?;
    }

    /// Adds new fuzz test. This means create new directory within the
    /// trident-tests/fuzz_tests directory, generate necessary files
    /// for fuzzing (instructions and snapshots) and modify
    /// trident-tests/fuzz_tests/Cargo.toml with the new generated
    /// fuzz test binary.
    #[throws]
    pub async fn add_fuzz_test(&mut self) {
        let root_path = self.root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        // build first
        commander.build_anchor_project().await?;
        // expand programs
        self.expand_programs().await?;
        // initialize fuzz tests if thay are not yet initialized
        self.add_new_fuzz_test().await?;
        // add trident.toml
        self.create_trident_manifest().await?;
        // update gitignore
        self.update_gitignore(CARGO_TARGET_DIR_DEFAULT)?;
    }
    /// Performs anchor build command and modify .program_client
    /// folder based on the updated program contents. If the .program_client
    /// is not yet generated, this will also generate the crate.
    #[throws]
    pub async fn build(&mut self) {
        let root_path = self.root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        // build first
        commander.build_anchor_project().await?;
        // expand programs
        self.expand_programs().await?;
        // obtain use statements
        // if program_client is not yet initialized
        // use statements are set to default
        self.get_program_client_imports().await?;
        // add/update program_client
        self.add_program_client().await?;
    }
    /// Collect program packages within the programs folder.
    /// Call rustc +nightly command in order to expand program macros, then parse
    /// the expanded code and obtain necessary data for generating test files
    #[throws]
    async fn expand_programs(&mut self) {
        self.packages = Commander::collect_program_packages().await?;
        self.programs_data = Commander::expand_program_packages(&self.packages).await?;
    }
    /// Get user specified use statements from .program_client lib.
    #[throws]
    async fn get_program_client_imports(&mut self) {
        let lib_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, SRC_DIRECTORY, LIB);
        if lib_path.exists() {
            let code = fs::read_to_string(lib_path).await.unwrap_or_else(|_e| {
                println!(
                    "{WARNING} Unable to read [.program_client], use statements set to default"
                );
                String::default()
            });
            Commander::get_use_statements(&code, &mut self.use_tokens)?;
        }
        if self.use_tokens.is_empty() {
            self.use_tokens
                .push(syn::parse_quote! { use trident_client::*; })
        }
    }

    /// Checks if the whole folder structure for .program_client is already
    /// present, if not create/update .program_client crate with the necessary files.
    #[throws]
    async fn init_program_client(&mut self) {
        let cargo_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, CARGO_TOML);
        let src_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, SRC_DIRECTORY);
        let crate_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY);
        let lib_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, SRC_DIRECTORY, LIB);

        if cargo_path.exists() && src_path.exists() && crate_path.exists() && lib_path.exists() {
            println!("{SKIP} looks like [.program_client] is already initialized");
        } else {
            self.add_program_client().await?;
        }
    }

    // Checks if whole Fuzz Test folder structer is already initialized,
    // and if fuzz_tests directory contains anything except Cargo.toml and fuzzing folder
    // if so the function does not proceed with Fuzz inicialization
    #[throws]
    async fn init_fuzz_tests(&mut self) {
        // create reuqired paths
        let fuzz_dir_path =
            construct_path!(self.root, TESTS_WORKSPACE_DIRECTORY, FUZZ_TEST_DIRECTORY);
        let fuzz_tests_manifest_path = construct_path!(fuzz_dir_path, CARGO_TOML);

        if fuzz_dir_path.exists() {
            // obtain directory contents
            let mut directories: std::collections::HashSet<_> = fuzz_dir_path
                .read_dir()
                .expect("Reading directory failed")
                .map(|r| {
                    r.expect("Reading directory; DirEntry error")
                        .file_name()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
            directories.retain(|x| x != "fuzzing");
            directories.retain(|x| x != CARGO_TOML);
            // if folder structure exists and fuzz_tests directory is not empty we skip
            if fuzz_tests_manifest_path.exists() && !directories.is_empty() {
                println!("{SKIP} looks like [Fuzz] Tests are already initialized");
            } else {
                self.add_new_fuzz_test().await?
            }
        } else {
            self.add_new_fuzz_test().await?
        }
    }

    // Checks if whole PoC Test folder structer is already initialized, if so
    // the function does not proceed with PoC inicialization
    #[throws]
    async fn init_poc_tests(&mut self) {
        // create reuqired paths

        let poc_dir_path =
            construct_path!(self.root, TESTS_WORKSPACE_DIRECTORY, POC_TEST_DIRECTORY);
        let new_poc_test_dir = construct_path!(poc_dir_path, TESTS_DIRECTORY);
        let cargo_path = construct_path!(poc_dir_path, CARGO_TOML);
        let poc_test_path = construct_path!(new_poc_test_dir, POC_TEST);

        // if folder structure exists we skip
        if poc_dir_path.exists()
            && new_poc_test_dir.exists()
            && cargo_path.exists()
            && poc_test_path.exists()
        {
            println!("{SKIP} looks like [PoC] Tests are already initialized");
        } else {
            self.add_new_poc_test().await?;
        }
    }

    /// Adds new PoC Test (This will Generate only one PoC Test file).
    /// If not present create trident-tests directory.
    /// If not present create poc_tests directory.
    /// If not present create tests directory.
    /// If not present generate PoC test file.
    /// If not present add program dependencies into the Cargo.toml file inside poc_tests folder
    /// If not present add poc_tests into the workspace virtual manifest as member
    #[throws]
    async fn add_new_poc_test(&self) {
        let program_name = if !&self.programs_data.is_empty() {
            &self
                .programs_data
                .first()
                .unwrap()
                .program_idl
                .name
                .snake_case
        } else {
            throw!(Error::NoProgramsFound)
        };
        let poc_dir_path =
            construct_path!(self.root, TESTS_WORKSPACE_DIRECTORY, POC_TEST_DIRECTORY);
        let new_poc_test_dir = construct_path!(poc_dir_path, TESTS_DIRECTORY);
        let cargo_path = construct_path!(poc_dir_path, CARGO_TOML);
        let poc_test_path = construct_path!(new_poc_test_dir, POC_TEST);

        // self.create_directory(&poc_dir_path).await?;
        self.create_directory_all(&new_poc_test_dir).await?;
        let cargo_toml_content = load_template!("/src/templates/trident-tests/Cargo_poc.toml.tmpl");
        self.create_file(&cargo_path, cargo_toml_content).await?;

        let poc_test_content = load_template!("/src/templates/trident-tests/test.rs");
        let test_content = poc_test_content.replace("###PROGRAM_NAME###", program_name);
        let use_instructions = format!("use program_client::{}_instruction::*;\n", program_name);
        let template = format!("{use_instructions}{test_content}");

        self.create_file(&poc_test_path, &template).await?;

        // add poc test to the workspace virtual manifest
        self.add_workspace_member(&format!("{TESTS_WORKSPACE_DIRECTORY}/{POC_TEST_DIRECTORY}",))
            .await?;

        // add program dev-dependencies into the poc tests Cargo
        // dev-deps are ok as they are used with the cargo test
        self.add_program_dependencies(&poc_dir_path, "dev-dependencies", None)
            .await?;
    }

    /// Adds new Fuzz Test.
    /// If not present create trident-tests directory.
    /// If not present create fuzz_tests directory.
    /// Obtain name for the new fuzz test and generate new fuzz test
    /// directory inside fuzz_tests folder.
    /// Generate fuzz test files and update Cargo.toml with the new Fuzz Test binary path.
    /// If not present add program dependencies into the Cargo.toml file inside fuzz_tests folder
    /// If not present add fuzz_tests into the workspace virtual manifest as member
    #[throws]
    pub async fn add_new_fuzz_test(&self) {
        let program_name = if !&self.programs_data.is_empty() {
            &self
                .programs_data
                .first()
                .unwrap()
                .program_idl
                .name
                .snake_case
        } else {
            throw!(Error::NoProgramsFound)
        };
        let fuzz_dir_path =
            construct_path!(self.root, TESTS_WORKSPACE_DIRECTORY, FUZZ_TEST_DIRECTORY);
        let fuzz_tests_manifest_path = construct_path!(fuzz_dir_path, CARGO_TOML);

        self.create_directory_all(&fuzz_dir_path).await?;

        let fuzz_id = if fuzz_dir_path.read_dir()?.next().is_none() {
            0
        } else {
            let mut directories: std::collections::HashSet<_> = fuzz_dir_path
                .read_dir()
                .expect("Reading directory failed")
                .map(|r| {
                    r.expect("Reading directory; DirEntry error")
                        .file_name()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();

            // INFO discard known entries created by framework, everything else
            // created by user will be taken as fuzz test.
            directories.retain(|x| x != "fuzzing");
            directories.retain(|x| x != "Cargo.toml");

            let mut fuzz_id = directories.len();
            loop {
                let fuzz_test = format!("fuzz_{fuzz_id}");
                if directories.contains(&fuzz_test) && fuzz_id < usize::MAX {
                    fuzz_id += 1;
                } else {
                    break fuzz_id;
                }
            }
        };

        let new_fuzz_test = format!("fuzz_{fuzz_id}");
        let new_fuzz_test_dir = fuzz_dir_path.join(&new_fuzz_test);
        let new_bin_target = format!("{new_fuzz_test}/test_fuzz.rs");

        self.create_directory(&new_fuzz_test_dir).await?;

        let fuzz_test_path = new_fuzz_test_dir.join(FUZZ_TEST);

        let fuzz_test_content = load_template!("/src/templates/trident-tests/test_fuzz.rs");

        let use_entry = format!("use {}::entry;\n", program_name);
        let use_instructions = format!("use {}::ID as PROGRAM_ID;\n", program_name);
        let use_fuzz_instructions = format!(
            "use fuzz_instructions::{}_fuzz_instructions::FuzzInstruction;\n",
            program_name
        );
        let template =
            format!("{use_entry}{use_instructions}{use_fuzz_instructions}{fuzz_test_content}");
        let fuzz_test_content = template.replace("###PROGRAM_NAME###", program_name);

        self.create_file(&fuzz_test_path, &fuzz_test_content)
            .await?;

        // create fuzz instructions file
        let fuzz_instructions_path = new_fuzz_test_dir.join(FUZZ_INSTRUCTIONS_FILE_NAME);
        let program_fuzzer = fuzzer::fuzzer_generator::generate_source_code(&self.programs_data);
        let program_fuzzer = Commander::format_program_code(&program_fuzzer).await?;

        self.create_file(&fuzz_instructions_path, &program_fuzzer)
            .await?;

        // // create accounts_snapshots file
        let accounts_snapshots_path = new_fuzz_test_dir.join(ACCOUNTS_SNAPSHOTS_FILE_NAME);
        let fuzzer_snapshots =
            generate_snapshots_code(&self.programs_data).map_err(Error::ReadProgramCodeFailed)?;
        let fuzzer_snapshots = Commander::format_program_code(&fuzzer_snapshots).await?;

        self.create_file(&accounts_snapshots_path, &fuzzer_snapshots)
            .await?;

        let cargo_toml_content =
            load_template!("/src/templates/trident-tests/Cargo_fuzz.toml.tmpl");

        self.create_file(&fuzz_tests_manifest_path, cargo_toml_content)
            .await?;

        self.add_bin_target(&fuzz_tests_manifest_path, &new_fuzz_test, &new_bin_target)
            .await?;
        self.add_program_dependencies(&fuzz_dir_path, "dependencies", None)
            .await?;

        self.add_workspace_member(&format!(
            "{TESTS_WORKSPACE_DIRECTORY}/{FUZZ_TEST_DIRECTORY}",
        ))
        .await?;
    }

    /// Add/Update .program_client
    /// If not present create .program_client directory.
    /// If not present create src directory.
    /// If not present create Cargo.toml file
    /// If not present add program dependencies into the Cargo.toml file inside .program_client folder
    /// Generate .program_client code.
    /// If not present add .program_client code
    /// If present update .program_client code
    #[throws]
    async fn add_program_client(&self) {
        let cargo_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, CARGO_TOML);
        let src_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, SRC_DIRECTORY);
        let crate_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY);
        let lib_path = construct_path!(self.root, PROGRAM_CLIENT_DIRECTORY, SRC_DIRECTORY, LIB);

        self.create_directory_all(&src_path).await?;

        // load template
        let cargo_toml_content = load_template!("/src/templates/program_client/Cargo.toml.tmpl");

        // if path exists the file will not be overwritten
        self.create_file(&cargo_path, cargo_toml_content).await?;

        self.add_program_dependencies(&crate_path, "dependencies", Some(vec!["no-entrypoint"]))
            .await?;

        let program_client =
            program_client_generator::generate_source_code(&self.programs_data, &self.use_tokens);
        let program_client = Commander::format_program_code(&program_client).await?;

        if lib_path.exists() {
            self.update_file(&lib_path, &program_client).await?;
        } else {
            self.create_file(&lib_path, &program_client).await?;
        }
    }

    /// If not present create Trident manifest with the templte.
    #[throws]
    async fn create_trident_manifest(&self) {
        let trident_toml_path = construct_path!(self.root, TRIDENT_TOML);
        let trident_toml_content = load_template!("/src/templates/Trident.toml.tmpl");
        self.create_file(&trident_toml_path, trident_toml_content)
            .await?;
    }
    /// Adds a new member to the Cargo workspace manifest (`Cargo.toml`).
    ///
    /// This function updates the `Cargo.toml` file located at the root of the Cargo workspace
    /// by adding a new member to the `members` array within the `[workspace]` table. If the specified member
    /// already exists in the `members` array, the function will skip the addition and print a message indicating
    /// that the member is already present. Otherwise, it will add the new member and print a success message.
    ///
    /// # Parameters
    /// - `&self`: A reference to the current instance of the TestGenerator struct that holds the workspace root path.
    /// - `member`: A string slice (`&str`) representing the path to the new member to be added. This path should be
    /// relative to the workspace root.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The `Cargo.toml` file cannot be found, read, or is not properly formatted.
    /// - The `Cargo.toml` file does not contain a `[workspace]` table or a `members` array within that table,
    /// and it cannot be created.
    ///
    /// The function uses `Error::CannotParseCargoToml` to indicate failures related to parsing or updating the
    /// `Cargo.toml` file.
    #[throws]
    async fn add_workspace_member(&self, member: &str) {
        let cargo = construct_path!(self.root, CARGO_TOML);
        let mut content: Value = fs::read_to_string(&cargo).await?.parse()?;
        let new_member = Value::String(String::from(member));

        let members = content
            .as_table_mut()
            .ok_or(Error::CannotParseCargoToml)?
            .entry("workspace")
            .or_insert(Value::Table(Table::default()))
            .as_table_mut()
            .ok_or(Error::CannotParseCargoToml)?
            .entry("members")
            .or_insert(Value::Array(vec![new_member.clone()]))
            .as_array_mut()
            .ok_or(Error::CannotParseCargoToml)?;

        match members.iter().find(|&x| x.eq(&new_member)) {
            Some(_) => {
                println!("{SKIP} [{CARGO_TOML}], already contains [{member}]")
            }
            None => {
                members.push(new_member);
                println!("{FINISH} [{CARGO_TOML}] updated with [{member}]");
                fs::write(cargo, content.to_string()).await?;
            }
        };
    }

    /// If not present creates a new directory and all missing
    /// parent directories on the specified path
    #[throws]
    async fn create_directory_all(&self, path: &PathBuf) {
        match path.exists() {
            true => {}
            false => {
                fs::create_dir_all(path).await?;
            }
        };
    }
    /// If not present creates directory with specified path
    #[throws]
    async fn create_directory(&self, path: &PathBuf) {
        match path.exists() {
            true => {}
            false => {
                fs::create_dir(path).await?;
            }
        };
    }
    /// If not present creates a new file with a given content on the specified path
    /// If file is present, skip
    #[throws]
    async fn create_file(&self, path: &PathBuf, content: &str) {
        let file = path.strip_prefix(&self.root).unwrap().to_str().unwrap();

        match path.exists() {
            true => {
                println!("{SKIP} [{file}] already exists")
            }
            false => {
                fs::write(path, content).await?;
                println!("{FINISH} [{file}] created");
            }
        };
    }
    /// If present update a new file with a given content on the specified path
    /// If file is not present, skip
    #[throws]
    async fn update_file(&self, path: &PathBuf, content: &str) {
        let file = path.strip_prefix(&self.root).unwrap().to_str().unwrap();
        match path.exists() {
            true => {
                fs::write(path, content).await?;
                println!("{FINISH} [{file}] updated");
            }
            false => {
                fs::write(path, content).await?;
                println!("{FINISH} [{file}] created");
            }
        };
    }

    /// Updates the `.gitignore` file by appending a specified path to ignore.
    ///
    /// This function checks if the given `ignored_path` is already listed in the `.gitignore` file at the root
    /// of the repository. If the path is not found, it appends the `ignored_path` to the file, ensuring that it
    /// is ignored by Git. If the `.gitignore` file does not exist or the path is already included, the function
    /// will skip the addition and print a message.
    ///
    /// # Parameters
    /// - `&self`: A reference to the current instance of the TestGenerator that holds the repository root path.
    /// - `ignored_path`: A string slice (`&str`) representing the path to be ignored by Git. This path should be
    /// relative to the repository root.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The `.gitignore` file exists but cannot be opened or read.
    /// - There is an error writing the new `ignored_path` to the `.gitignore` file.
    #[throws]
    fn update_gitignore(&self, ignored_path: &str) {
        let gitignore_path = construct_path!(self.root, GIT_IGNORE);
        if gitignore_path.exists() {
            let file = File::open(&gitignore_path)?;
            for line in io::BufReader::new(file).lines().flatten() {
                if line == ignored_path {
                    // INFO do not add the ignored path again if it is already in the .gitignore file
                    println!("{SKIP} [{GIT_IGNORE}], already contains [{ignored_path}]");

                    return;
                }
            }
            let file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(gitignore_path);

            if let Ok(mut file) = file {
                writeln!(file, "{}", ignored_path)?;
                println!("{FINISH} [{GIT_IGNORE}] update with [{ignored_path}]");
            }
        } else {
            println!("{SKIP} [{GIT_IGNORE}], not found")
        }
    }
    /// Adds a new binary target to a Cargo.toml file.
    ///
    /// This function reads the existing `Cargo.toml` file from the specified path, adds a new binary target
    /// configuration to it, and writes the updated content back to the file. It handles the creation of a new
    /// `[[bin]]` section if one does not already exist or appends the new binary target to the existing `[[bin]]`
    /// array. The new binary target is specified by its name and the path to its source file.
    ///
    /// # Parameters
    /// - `&self`: A reference to the current instance of the TestGenerator, not used directly in this function but
    /// necessary for method calls on the instance.
    /// - `cargo_path`: A reference to a `PathBuf` that specifies the path to the `Cargo.toml` file to be updated.
    /// - `name`: A string slice (`&str`) representing the name of the binary target to be added.
    /// - `path`: A string slice (`&str`) representing the path to the source file of the binary target, relative
    /// to the Cargo package's root.
    ///
    /// # Errors
    /// This function returns an error if:
    /// - The `Cargo.toml` file cannot be read or written to.
    /// - The content of `Cargo.toml` cannot be parsed into a `toml::Value` or manipulated as expected.
    #[throws]
    async fn add_bin_target(&self, cargo_path: &PathBuf, name: &str, path: &str) {
        // Read the existing Cargo.toml file
        let cargo_toml_content = fs::read_to_string(cargo_path).await?;
        let mut cargo_toml: Value = cargo_toml_content.parse()?;

        // Create a new bin table
        let mut bin_table = Table::new();
        bin_table.insert("name".to_string(), Value::String(name.to_string()));
        bin_table.insert("path".to_string(), Value::String(path.to_string()));

        // Add the new [[bin]] section to the [[bin]] array
        if let Some(bin_array) = cargo_toml.as_table_mut().and_then(|t| t.get_mut("bin")) {
            if let Value::Array(bin_array) = bin_array {
                bin_array.push(Value::Table(bin_table));
            }
        } else {
            // If there is no existing [[bin]] array, create one
            let bin_array = Value::Array(vec![Value::Table(bin_table)]);
            cargo_toml
                .as_table_mut()
                .unwrap()
                .insert("bin".to_string(), bin_array);
        }

        // Write the updated Cargo.toml file
        fs::write(cargo_path, cargo_toml.to_string()).await?;
    }
    /// Adds program dependencies to a specified Cargo.toml file.
    ///
    /// This function updates the Cargo.toml file located in the given directory by adding new dependencies
    /// specified by the `deps` parameter. It supports adding dependencies with or without features. The
    /// dependencies are added based on the packages found in the `self.packages` collection of the TestGenerator,
    /// where each package's path is adjusted to be relative to the specified `cargo_dir`. If no packages are
    /// found in `self.packages`, the function will return an error.
    ///
    /// # Parameters
    /// - `&self`: A reference to the current instance of the TestGenerator, which contains a collection of packages
    /// to be added as dependencies.
    /// - `cargo_dir`: A reference to a `PathBuf` indicating the directory where the `Cargo.toml` file to be updated is located.
    /// - `deps`: A string slice (`&str`) specifying the section under which the dependencies should be added
    /// (e.g., `dependencies`, `dev-dependencies`, etc.).
    /// - `features`: An optional vector of string slices (`Vec<&str>`) specifying the features that should be
    /// enabled for the dependencies being added. If `None`, no features are specified.
    ///
    /// # Errors
    /// This function can return errors in several cases, including:
    /// - If the specified `Cargo.toml` file cannot be read or written to.
    /// - If parsing of the `Cargo.toml` file or the dependencies fails.
    /// - If no packages are found in `self.packages`.
    #[throws]
    async fn add_program_dependencies(
        &self,
        cargo_dir: &PathBuf,
        deps: &str,
        features: Option<Vec<&str>>,
    ) {
        let cargo_path = construct_path!(cargo_dir, "Cargo.toml");

        let mut cargo_toml_content: toml::Value = fs::read_to_string(&cargo_path).await?.parse()?;

        let client_toml_deps = cargo_toml_content
            .get_mut(deps)
            .and_then(toml::Value::as_table_mut)
            .ok_or(Error::ParsingCargoTomlDependenciesFailed)?;

        if self.packages.is_empty() {
            throw!(Error::NoProgramsFound)
        }
        for package in self.packages.iter() {
            let manifest_path = package.manifest_path.parent().unwrap().as_std_path();
            // INFO this will obtain relative path
            let relative_path = pathdiff::diff_paths(manifest_path, cargo_dir).unwrap();
            let dep: Value = if features.is_some() {
                format!(
                    r#"{} = {{ path = "{}", features = {:?} }}"#,
                    package.name,
                    relative_path.to_str().unwrap(),
                    features.as_ref().unwrap()
                )
                .parse()
                .unwrap()
            } else {
                format!(
                    r#"{} = {{ path = "{}" }}"#,
                    package.name,
                    relative_path.to_str().unwrap()
                )
                .parse()
                .unwrap()
            };
            if let toml::Value::Table(table) = dep {
                let (name, value) = table.into_iter().next().unwrap();
                client_toml_deps.entry(name).or_insert(value.clone());
            }
        }
        fs::write(cargo_path, cargo_toml_content.to_string()).await?;
    }
}
