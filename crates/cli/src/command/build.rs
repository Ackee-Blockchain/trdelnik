use anyhow::Error;
use fehler::throws;
use trdelnik_sandbox_client::*;

#[throws]
pub async fn build(root: String) {
    let commander = Commander::with_root(root);
    commander.create_program_client_crate().await?;
    commander.build_programs().await?;
    commander.generate_program_client_deps().await?;
    commander.generate_program_client_lib_rs().await?;
}
