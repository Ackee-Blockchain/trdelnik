<div align="center">
  <img height="250" width="250" src="https://github.com/Ackee-Blockchain/trdelnik/raw/master/assets/Badge_Trdelnik.png" alt="Trdelnik Logo"/>

  # Trdelník

  <a href="https://discord.gg/x7qXXnGCsa">
    <img src="https://discordapp.com/api/guilds/867746290678104064/widget.png?style=banner2" width="250" title="AckeeBlockchain/Trdelnik discord" alt="Ackee Blockchain Discord invitation">
  </a>

  developed by [Ackee Blockchain](https://ackeeblockchain.com)

  [![Crates.io](https://img.shields.io/crates/v/trdelnik-cli?label=CLI)](https://crates.io/crates/trdelnik-cli)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-test?label=Test)](https://crates.io/crates/trdelnik-test)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-client?label=Client)](https://crates.io/crates/trdelnik-client)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-explorer?label=Explorer)](https://crates.io/crates/trdelnik-explorer)
  <br />
  [![lint](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/lint.yml/badge.svg)](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/lint.yml)
  [![Test Escrow and Turnstile](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/run_examples.yml/badge.svg)](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/run_examples.yml)
</div>

Trdelník is Rust based testing framework providing several convenient developer tools for testing Solana programs written in [Anchor](https://github.com/project-serum/anchor).

- **Trdelnik fuzz** - property-based and stateful testing;
- Trdelnik client - build and deploy an Anchor program to a local cluster and run a test suite against it;
- Trdelnik console - built-in console to give developers a command prompt for quick program interaction;
- Trdelnik explorer - exploring a ledger changes.

## Dependencies

- Install [Rust](https://www.rust-lang.org/tools/install) (`nightly` release)
- Install [Solana tool suite](https://docs.solana.com/cli/install-solana-cli-tools) (`stable` release)
- Install [Anchor](https://book.anchor-lang.com/chapter_2/installation.html)
- Optionally install [Honggfuzz-rs](https://github.com/rust-fuzz/honggfuzz-rs#how-to-use-this-crate) for fuzz testing

## Installation

```shell
cargo install trdelnik-cli

# or the specific version

cargo install --version <version> trdelnik-cli
```

## Usage

```shell
# Navigate to your project root directory.
# Trdelnik initialization will generate `.program_client` and `trdelnik-tests` directories with all the necessary files.
trdelnik init
# Run the fuzzer on the given target.
trdelnik fuzz run <TARGET_NAME>
# Want more?
trdelnik --help
```
### How to write fuzz tests?
Once you initialize Trdelnik in your Anchor project, you will find a fuzz test template in the `trdelnik-tests/src/bin` folder that you can modify according to your needs or create new targets. Do not forget to install honggfuzz-rs using `cargo install honggfuzz`.


```shell
# To run the fuzz test, execute this command from your terminal and replace <TARGET_NAME> with the name of your fuzz target (by default "fuzz_target")
trdelnik fuzz run <TARGET_NAME>

# To debug your fuzz target crash with parameters from a crash file
trdelnik fuzz run-debug <TARGET_NAME> <CRASH_FILE_PATH>
```

 Under the hood Trdelnik uses [honggfuzz-rs](https://github.com/rust-fuzz/honggfuzz-rs). You can pass parameters via [environment variables](https://github.com/rust-fuzz/honggfuzz-rs#environment-variables). List of hongfuzz parameters can be found in honggfuzz [usage documentation](https://github.com/google/honggfuzz/blob/master/docs/USAGE.md#cmdline---help). For example:
 ```shell
# Time-out: 10 secs
# Number of concurrent fuzzing threads: 1
# Number of fuzzing iterations: 10000
# Display Solana logs in the terminal
HFUZZ_RUN_ARGS="-t 10 -n 1 -N 10000 -Q" trdelnik fuzz run <TARGET_NAME>
```

> NOTE: If you will use the `solana-program-test` crate for fuzzing, creating a new test program using `ProgramTest::new()` will create temporary folders in your `/tmp` directory that will not be cleared in case your program panics. You might want to clear these folders manually.

### How to write tests?
Trdelnik also supports writing integration tests in Rust.

<div align="center">
  <img src="https://github.com/Ackee-Blockchain/trdelnik/raw/master/assets/demo.svg" alt="Trdelnik Demo" />
</div>

```rust
// <my_project>/trdelnik-tests/tests/test.rs
// TODO: do not forget to add all necessary dependencies to the generated `trdelnik-tests/Cargo.toml`
use program_client::my_instruction;
use trdelnik_client::*;
use my_program;

#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
  // create a test fixture
  let mut fixture = Fixture {
    client: Client::new(system_keypair(0)),
    // make sure your program is using a correct program ID
    program: program_keypair(1),
    state: keypair(42),
  };
  // deploy a tested program
  fixture.deploy().await?;
  // call instruction init
  my_instruction::initialize(
    &fixture.client,
    fixture.state.pubkey(),
    fixture.client.payer().pubkey(),
    System::id(),
    Some(fixture.state.clone()),
  ).await?;
  fixture
}

#[trdelnik_test]
async fn test_happy_path(#[future] init_fixture: Result<Fixture>) {
  let fixture = init_fixture.await?;
  // call the instruction
  my_instruction::do_something(
    &fixture.client,
    "dummy_string".to_owned(),
    fixture.state.pubkey(),
    None,
  ).await?;
  // check the test result
  let state = fixture.get_state().await?;
  assert_eq!(state.something_changed, "yes");
}
```

Make sure your program is using a correct program ID in the `derive_id!(...)` macro and inside `Anchor.toml`.
If not, obtain the public key of a key pair you're using and replace it in these two places.
To get the program ID of a key pair (key pair's public key) the `trdelnik key-pair` command can be used.
For example
```
$ trdelnik key-pair program 7
```
will print information about the key pair received from `program_keypair(7)`.

#### Instructions with custom structures

- If you want to test an instruction which has custom structure as an argument

```rust
pub struct MyStruct {
  amount: u64,
}

// ...

pub fn my_instruction(ctx: Context<Ctx>, data: MyStruct) { /* ... */ }
```

- You should add an import to the `.program_client` crate

```rust
// .program_client/src/lib.rs

// DO NOT EDIT - automatically generated file
pub mod my_program_instruction {
  use trdelnik_client::*;
  use my_program::MyStruct; // add this import

// ...
}
```

- This file is automatically generated but the **`use` statements won't be regenerated**

#### Skipping tests

- You can add the `#[ignore]` macro to skip the test.

```rust
#[trdelnik_test]
#[ignore]
async fn test() {}
```

#### Testing programs with associated token accounts

- `Trdelnik` does not export `anchor-spl` and `spl-associated-token-account`, so you have to add it manually.

```toml
# <my-project>/trdelnik-tests/Cargo.toml
# import the correct versions manually
anchor-spl = "0.28.0"
spl-associated-token-account = "2.0.0"
```

```rust
// <my-project>/trdelnik-tests/tests/test.rs
use anchor_spl::token::Token;
use spl_associated_token_account;

async fn init_fixture() -> Fixture {
  // ...
  let account = keypair(1);
  let mint = keypair(2);
  // constructs a token mint
  client
    .create_token_mint(&mint, mint.pubkey(), None, 0)
    .await?;
  // constructs associated token account
  let token_account = client
    .create_associated_token_account(&account, mint.pubkey())
    .await?;
  let associated_token_program = spl_associated_token_account::id();
  // derives the associated token account address for the given wallet and mint
  let associated_token_address = spl_associated_token_account::get_associated_token_address(&account.pubkey(), mint);
  Fixture {
    // ...
    token_program: Token::id(),
  }
}
```

- The `trdelnik init` command generated a dummy test suite for you.
- For more details, see the [complete test](examples/turnstile/trdelnik-tests/tests/test.rs) implementation.


### Supported versions

- We support `Anchor` and `Solana` versions specified in the table below.

| Trdelnik CLI |  Anchor   |   Solana |
|--------------|:---------:|---------:|
| `latest`     | `~0.28.*` | `=1.16.6` |
| `v0.4.0`     | `~0.27.*` | `>=1.15` |
| `v0.3.0`     | `~0.25.*` | `>=1.10` |
| `v0.2.0`     | `~0.24.*` |  `>=1.9` |

- _We are exploring a new versions of Anchor, please make sure you only use the supported versions. We are working on it :muscle:_

### Configuration

The configuration variables can be edited in the `Trdelnik.toml` file that'll be generated in the root of the project.

| Name                             | Default value | Description                                                                 |
|----------------------------------|---------------|-----------------------------------------------------------------------------|
| `test.validator_startup_timeout` | 10 000        | Time to wait for the `solana-test-validator` in milliseconds before failure |

## Roadmap

- [x] Q1/22 Trdelnik announcement at Solana Hacker House Prague
  - [x] Trdelnik client available for testing
- [x] Q2/22 Trdelnik explorer available
- [x] Q2/22 Trdelnik client and explorer introduced at Solana Hacker House Barcelona
- [ ] Q3/23 Trdelnik fuzz introduced at Solana Hacker House Berlin

## Awards

**Marinade Community Prize** - winner of the [Marinade grant](https://solana.blog/riptide-hackathon-winners/) for the 2022 Solana Riptide Hackathon.

## Contribution

Thank you for your interest in contributing to Trdelník! Please see the [CONTRIBUTING.md](./CONTRIBUTING.md) to learn how.

## License

This project is licensed under the [MIT license](https://github.com/Ackee-Blockchain/trdelnik/blob/master/LICENSE).

## University and investment partners

- [Czech technical university in Prague](https://www.cvut.cz/en)
- [Ackee](https://www.ackee.cz/)
- [Rockaway Blockchain Fund](https://rbf.capital/)
