use anyhow::{bail, Error};
use fehler::throws;
use trdelnik_client::WorkspaceBuilder;

use crate::discover;
pub const TRDELNIK_TOML: &str = "Trdelnik.toml";

#[throws]
pub async fn clean() {
    let root = if let Some(r) = discover(TRDELNIK_TOML)? {
        r
    } else {
        bail!("It does not seem that Trdelnik is initialized because the Trdelnik.toml file was not found in any parent directory!");
    };
    let builder = WorkspaceBuilder::new_with_root(root);
    builder.clean().await?;
}
