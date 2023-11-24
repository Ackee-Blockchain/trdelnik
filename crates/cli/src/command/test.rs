use anyhow::Error;
use fehler::throws;
use trdelnik_client::*;

#[throws]
pub async fn test() {
    Commander::run_tests().await?;
}
