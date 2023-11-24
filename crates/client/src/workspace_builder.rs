use crate::constants::*;
use crate::{
    commander::{Commander, Error as CommanderError},
    config::{CARGO_TOML, TRDELNIK_TOML},
    idl::Idl,
    program_client_generator,
};
use fehler::{throw, throws};
use std::{
    env,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
};
use std::{fs::File, io::prelude::*};
use syn::ItemUse;
use thiserror::Error;
use tokio::fs;
use toml::{
    value::{Map, Table},
    Value,
};

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
    #[error("The Anchor project does not contain any programs")]
    NoProgramsFound,
    #[error("parsing Cargo.toml dependencies failed")]
    ParsingCargoTomlDependenciesFailed,
}

pub struct WorkspaceBuilder {
    root: PathBuf,
    idl: Option<Idl>,
    use_tokens: Option<Vec<ItemUse>>,
}

impl WorkspaceBuilder {
    pub fn new_with_root(root: String) -> Self {
        Self {
            root: Path::new(&root).to_path_buf(),
            idl: None,
            use_tokens: None,
        }
    }

    #[throws]
    pub async fn clean(&self) {
        Commander::clean_anchor_target().await?;
        Commander::clean_hfuzz_target(&self.root).await?;
    }

    #[throws]
    pub async fn build(&mut self) {
        self.build_and_parse().await?;

        // automatically generated so we are ok with updating this
        self.update_program_client().await?;
        self.update_program_stubs().await?;
        self.update_toml_dependencies().await?;
        self.add_invoked_program_deps().await?;
    }
    #[throws]
    pub async fn generate_with_fuzzer(&mut self) {
        // build first
        self.build_and_parse().await?;

        self.create_program_client_crate().await?;
        self.create_trdelnik_tests_crate().await?;
        self.create_trdelnik_tests().await?;
        self.create_trdelnik_fuzz().await?;

        self.update_toml_dependencies().await?;
        self.add_invoked_program_deps().await?;

        self.create_trdelnik_manifest().await?;
        self.update_workspace_cargo().await?;
        self.update_gitignore("hfuzz_target")?;
    }

    #[throws]
    pub async fn generate_without_fuzzer(&mut self) {
        // build first
        self.build_and_parse().await?;

        self.create_program_client_crate().await?;
        self.create_trdelnik_tests_crate().await?;
        self.create_trdelnik_tests().await?;

        self.update_toml_dependencies().await?;

        self.create_trdelnik_manifest().await?;
        self.update_workspace_cargo().await?;
    }
    #[throws]
    async fn build_and_parse(&mut self) {
        Commander::build_programs().await?;

        println!("--> Obtaining data from programs ... <--");
        self.idl = Commander::obtain_program_idl(&self.root).await?;
        self.use_tokens = Commander::parse_program_client_imports().await?;
        println!("\x1b[92mSuccesfully\x1b[0m obtained.");
    }
    /// Creates .program_client folder, subfolders and files
    #[throws]
    async fn create_program_client_crate(&self) {
        let crate_path = self.root.join(PROGRAM_CLIENT_DIRECTORY);
        let cargo_path = crate_path.join(CARGO);
        let src_path = crate_path.join(SRC);
        let lib_path = src_path.join(LIB);

        self.create_directory_all(&src_path).await?;

        let cargo_toml_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/program_client/Cargo.toml.tmpl"
        ));
        self.create_file(&cargo_path, cargo_toml_content).await?;

        let program_client = program_client_generator::generate_source_code(
            self.idl.as_ref().unwrap(),
            self.use_tokens.as_ref().unwrap(),
        );
        let program_client = Commander::format_program_code(&program_client).await?;

        self.create_file(&lib_path, &program_client).await?;
    }
    /// Creates trdelnik-tests crate
    #[throws]
    async fn create_trdelnik_tests_crate(&self) {
        let workspace_path = self.root.join(TESTS_WORKSPACE_DIRECTORY);
        let cargo_toml_path = workspace_path.join(CARGO_TOML);

        self.create_directory(&workspace_path).await?;

        let cargo_toml_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/trdelnik-tests/Cargo.toml.tmpl"
        ));
        self.create_file(&cargo_toml_path, cargo_toml_content)
            .await?;
    }

    #[throws]
    async fn create_trdelnik_tests(&self) {
        let tests_path = self
            .root
            .join(TESTS_WORKSPACE_DIRECTORY)
            .join(TEST_DIRECTORY);
        let test_path = tests_path.join(TEST);

        self.create_directory(&tests_path).await?;

        let test_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/trdelnik-tests/test.rs"
        ));

        match &self.idl {
            Some(idl) => {
                let program_name = &idl.programs.first().unwrap().name.snake_case;
                let test_content = test_content.replace("###PROGRAM_NAME###", program_name);
                let use_instructions =
                    format!("use program_client::{}_instruction::*;\n", program_name);
                let template = format!("{use_instructions}{test_content}");

                self.create_file(&test_path, &template).await?;
            }
            None => {
                throw!(Error::NoProgramsFound)
            }
        }
    }

    #[throws]
    async fn create_trdelnik_manifest(&self) {
        let trdelnik_toml_path = self.root.join(TRDELNIK_TOML);
        let trdelnik_toml_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/Trdelnik.toml.tmpl"
        ));
        self.create_file(&trdelnik_toml_path, trdelnik_toml_content)
            .await?;
    }

    /// Creates the `trdelnik-tests` workspace with `src/bin` directory and empty `fuzz_target.rs` file
    #[throws]
    async fn create_trdelnik_fuzz(&self) {
        let fuzzer_path = self
            .root
            .join(TESTS_WORKSPACE_DIRECTORY)
            .join(FUZZ_DIRECTORY);
        let program_stubs_path = fuzzer_path.join(PROGRAM_STUBS);
        let fuzzer_test_path = fuzzer_path.join(FUZZ);

        self.create_directory_all(&fuzzer_path).await?;

        let fuzz_test_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/trdelnik-tests/fuzz_target.rs"
        ))
        .to_string();

        let program_stubs_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/trdelnik-tests/program_stubs.rs"
        ))
        .to_string();

        match &self.idl {
            Some(idl) => {
                let fuzz_test_content = get_fuzz_test_content(idl, fuzz_test_content)?;
                self.create_file(&fuzzer_test_path, &fuzz_test_content)
                    .await?;

                let program_stubs_content = get_program_stubs_content(idl, program_stubs_content)?;
                self.create_file(&program_stubs_path, &program_stubs_content)
                    .await?;
            }
            None => {
                throw!(Error::NoProgramsFound)
            }
        }

        self.add_feature_to_dep("trdelnik-client", "fuzzing")
            .await?;
    }

    #[throws]
    async fn update_toml_dependencies(&self) {
        let client_deps = self.root.join(PROGRAM_CLIENT_DIRECTORY).join("Cargo.toml");
        let test_deps = self
            .root
            .join(crate::constants::TESTS_WORKSPACE_DIRECTORY)
            .join("Cargo.toml");

        let mut client_toml_content: toml::Value =
            fs::read_to_string(&client_deps).await?.parse()?;
        let mut test_toml_content: toml::Value = fs::read_to_string(&test_deps).await?.parse()?;

        let client_toml_deps = client_toml_content
            .get_mut("dependencies")
            .and_then(toml::Value::as_table_mut)
            .ok_or(Error::ParsingCargoTomlDependenciesFailed)?;

        let test_toml_deps = test_toml_content
            .get_mut("dependencies")
            .and_then(toml::Value::as_table_mut)
            .ok_or(Error::ParsingCargoTomlDependenciesFailed)?;

        match &self.idl {
            Some(idl) => {
                for program in idl.programs.iter() {
                    let dep_client: Value = format!(
                        r#"{} = {{ path = "../{}", features = ["no-entrypoint"] }}"#,
                        program.name.snake_case,
                        program.path.to_str().unwrap()
                    )
                    .parse()
                    .unwrap();
                    let dep_test: Value = format!(
                        r#"{} = {{ path = "../{}" }}"#,
                        program.name.snake_case,
                        program.path.to_str().unwrap()
                    )
                    .parse()
                    .unwrap();

                    if let toml::Value::Table(table) = dep_client {
                        let (name, value) = table.into_iter().next().unwrap();
                        client_toml_deps.entry(name).or_insert(value.clone());
                    }
                    if let toml::Value::Table(table) = dep_test {
                        let (name, value) = table.into_iter().next().unwrap();
                        test_toml_deps.entry(name).or_insert(value.clone());
                    }
                }
                fs::write(client_deps, client_toml_content.to_string()).await?;
                fs::write(test_deps, test_toml_content.to_string()).await?;
            }
            None => {
                throw!(Error::NoProgramsFound)
            }
        }
    }

    #[throws]
    async fn update_program_client(&self) {
        let lib_path = self
            .root
            .join(PROGRAM_CLIENT_DIRECTORY)
            .join("src")
            .join("lib.rs");

        let program_client = program_client_generator::generate_source_code(
            self.idl.as_ref().unwrap(),
            self.use_tokens.as_ref().unwrap(),
        );
        let program_client = Commander::format_program_code(&program_client).await?;

        self.update_file(&lib_path, &program_client).await?;
    }

    /// Creates a new file with a given content on the specified path
    /// Skip if file already exists
    #[throws]
    async fn create_file<'a>(&self, path: &'a Path, content: &str) {
        let file = path.strip_prefix(&self.root).unwrap().to_str().unwrap();

        match path.exists() {
            true => {
                println!("\x1b[93m--> Skipping <--\x1b[0m \x1b[93m{file}\x1b[0m, already exists.")
            }
            false => {
                fs::write(path, content).await?;
                println!("\x1b[92mSuccesfully\x1b[0m created: \x1b[93m{file}\x1b[0m.");
            }
        };
    }

    /// Updates a file with a given content on the specified path
    /// Skip if file does not exists
    #[throws]
    async fn update_file<'a>(&self, path: &'a Path, content: &str) {
        let file = path.strip_prefix(&self.root).unwrap().to_str().unwrap();
        match path.exists() {
            true => {
                fs::write(path, content).await?;
                println!("\x1b[92mSuccesfully\x1b[0m updated: \x1b[93m{file}\x1b[0m.");
            }
            false => {
                println!("\x1b[93m--> Skipping <--\x1b[0m \x1b[93m{file}\x1b[0m, does not exists.");
            }
        };
    }

    /// Creates directory with specified path
    #[throws]
    async fn create_directory<'a>(&self, path: &'a Path) {
        match path.exists() {
            true => {}
            false => {
                fs::create_dir(path).await?;
            }
        };
    }
    /// Creates a new directory and all missing parent directories on the specified path
    #[throws]
    async fn create_directory_all<'a>(&self, path: &'a PathBuf) {
        match path.exists() {
            true => {}
            false => {
                fs::create_dir_all(path).await?;
            }
        };
    }

    #[throws]
    async fn update_program_stubs(&self) {
        let program_stubs_path = self
            .root
            .join(TESTS_WORKSPACE_DIRECTORY)
            .join(FUZZ_DIRECTORY)
            .join(PROGRAM_STUBS);

        let program_stubs_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/trdelnik-tests/program_stubs.rs"
        ))
        .to_string();

        match &self.idl {
            Some(idl) => {
                let program_stubs_content = get_program_stubs_content(idl, program_stubs_content)?;
                self.update_file(&program_stubs_path, &program_stubs_content)
                    .await?;
            }
            None => {
                throw!(Error::NoProgramsFound)
            }
        }
    }

    /// Adds `trdelnik-tests` workspace to the `root`'s `Cargo.toml` workspace members if needed.
    #[throws]
    async fn update_workspace_cargo(&self) {
        let cargo = self.root.join(CARGO_TOML);
        let mut content: Value = fs::read_to_string(&cargo).await?.parse()?;
        let test_workspace_value = Value::String(String::from(TESTS_WORKSPACE_DIRECTORY));
        let members = content
            .as_table_mut()
            .ok_or(Error::CannotParseCargoToml)?
            .entry("workspace")
            .or_insert(Value::Table(Table::default()))
            .as_table_mut()
            .ok_or(Error::CannotParseCargoToml)?
            .entry("members")
            .or_insert(Value::Array(vec![test_workspace_value.clone()]))
            .as_array_mut()
            .ok_or(Error::CannotParseCargoToml)?;
        match members.iter().find(|&x| x.eq(&test_workspace_value)) {
            Some(_) => {
                println!(
                    "\x1b[93m--> Skipping <--\x1b[0m \x1b[93m{CARGO_TOML}\x1b[0m, already contains trdelnik-tests."
                )
            }
            None => {
                members.push(test_workspace_value);
                println!("\x1b[92mSuccesfully\x1b[0m updated: \x1b[93m{CARGO_TOML}\x1b[0m.");
            }
        };
        fs::write(cargo, content.to_string()).await?;
    }

    /// Updates .gitignore file in the `root` directory and appends `ignored_path` to the end of the file
    #[throws]
    fn update_gitignore(&self, ignored_path: &str) {
        let file_path = self.root.join(GIT_IGNORE);
        if file_path.exists() {
            let file = File::open(&file_path)?;
            for line in io::BufReader::new(file).lines().flatten() {
                if line == ignored_path {
                    // do not add the ignored path again if it is already in the .gitignore file
                    return;
                }
            }
            let file = OpenOptions::new().write(true).append(true).open(file_path);

            if let Ok(mut file) = file {
                writeln!(file, "{}", ignored_path)?;
                println!("\x1b[92mSuccesfully\x1b[0m updated: \x1b[93m{GIT_IGNORE}\x1b[0m.");
            }
        } else {
            println!("\x1b[93m--> Skipping <--\x1b[0m \x1b[93m{GIT_IGNORE}\x1b[0m, not found.")
        }
    }

    /// Adds programs to Cargo.toml as a dependencies to be able to be used in tests and fuzz targets
    #[throws]
    async fn add_invoked_program_deps(&self) {
        let cargo_toml_path = self.root.join(TESTS_WORKSPACE_DIRECTORY).join(CARGO);

        match &self.idl {
            Some(idl) => {
                let mut content: Value = fs::read_to_string(&cargo_toml_path).await?.parse()?;
                let deps: &mut Map<String, Value> = content
                    .get_mut("dependencies")
                    .and_then(Value::as_table_mut)
                    .ok_or(Error::CannotParseCargoToml)?;

                for program in idl.programs.iter() {
                    for x in program.program_invocations.iter() {
                        if PROCESS_INSTRUCTIONS.contains_key(x.as_str()) {
                            let name = PROCESS_INSTRUCTIONS.get(x.as_str()).unwrap().2;
                            let version = PROCESS_INSTRUCTIONS.get(x.as_str()).unwrap().3;
                            let version = Value::String(version.to_string());
                            deps.entry(name).or_insert(version);
                        }
                    }
                }
                fs::write(cargo_toml_path, content.to_string()).await?;
            }
            None => {
                throw!(Error::NoProgramsFound)
            }
        }
    }

    /// Adds programs to Cargo.toml as a dev dependencies to be able to be used in tests
    #[throws]
    async fn add_feature_to_dep(&self, dependency: &str, feature: &str) {
        let cargo_toml_path = self.root.join(TESTS_WORKSPACE_DIRECTORY).join(CARGO_TOML);
        let rel_path = &cargo_toml_path
            .strip_prefix(&self.root)
            .unwrap()
            .to_str()
            .unwrap();
        let mut content: Value = fs::read_to_string(&cargo_toml_path).await?.parse()?;
        let deps = content
            .get_mut("dependencies")
            .and_then(Value::as_table_mut)
            .ok_or(Error::CannotParseCargoToml)?;

        let values = deps
            .get_mut(dependency)
            .and_then(|f| {
                if f.is_table() {
                    f.as_table_mut()
                } else if f.is_str() {
                    // if the value is only a string with version such as dependency = 0.0, create a new table with that version
                    let version = f.as_str().unwrap();
                    let mut map = Map::new();
                    let _ = map.insert("version".to_string(), Value::String(version.to_string()));
                    let t = Value::Table(map);
                    *f = t.to_owned();
                    f.as_table_mut()
                } else {
                    None
                }
            })
            .ok_or(Error::CannotParseCargoToml)?;

        let fuzzing = Value::String(feature.to_string());
        let value = Value::Array(vec![]);
        let features = values.entry("features").or_insert(value);
        if let Some(features) = features.as_array_mut() {
            if !features.iter().any(|f| *f == fuzzing) {
                features.push(fuzzing);
                fs::write(&cargo_toml_path, content.to_string()).await?;
                println!("\x1b[92mSuccesfully\x1b[0m updated: \x1b[93m{rel_path}\x1b[0m {feature} feature added.");
            } else {
                println!("\x1b[93m--> Skipping <--\x1b[0m \x1b[93m{rel_path}\x1b[0m, already contains {feature} feature.")
            }
        }
    }
}

#[throws]
fn get_fuzz_test_content(idl: &Idl, content: String) -> String {
    let name = &idl.programs.first().unwrap().name.snake_case;
    let use_entry = format!("use {}::entry;\n", name);
    let use_instructions = format!("use program_client::{}_instruction::*;\n", name);
    let template = format!("{use_entry}{use_instructions}{content}");
    template.replace("###PROGRAM_NAME###", name)
}

#[throws]
fn get_program_stubs_content(idl: &Idl, content: String) -> String {
    let mut final_content = content;
    for program in idl.programs.iter() {
        for x in program.program_invocations.iter() {
            if PROCESS_INSTRUCTIONS.contains_key(x.as_str()) {
                let function = PROCESS_INSTRUCTIONS.get(x.as_str()).unwrap().1;
                let program_id = PROCESS_INSTRUCTIONS.get(x.as_str()).unwrap().0;

                let replacement_string = format!(
                    "create_processor!({},{});\n\t\t{}",
                    function, program_id, PROGRAM_STUBS_ENTRIES
                );
                final_content = final_content.replace(PROGRAM_STUBS_ENTRIES, &replacement_string);
            }
        }
    }
    final_content
}
