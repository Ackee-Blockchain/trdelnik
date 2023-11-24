use anyhow::{bail, Error};
use fehler::throws;
use trdelnik_client::*;

use crate::discover;

pub const TRDELNIK_TOML: &str = "Trdelnik.toml";

#[throws]
pub async fn build(root: Option<String>) {
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
    let mut builder = WorkspaceBuilder::new_with_root(root);
    builder.build().await?;
}
