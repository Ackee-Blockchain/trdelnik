use crate::{commander::Error, Commander, LocalnetHandle};
use fehler::throws;
use log::debug;
use std::path::{Path, PathBuf};

/// `Tester` is used primarily by [`#[trdelnik_test]`](trdelnik_test::trdelnik_test) macro.
///
/// There should be no need to use `Tester` directly.
#[derive(Default)]
pub struct Tester {
    root: PathBuf,
}

impl Tester {
    pub fn new() -> Self {
        Self {
            root: "../../".into(),
        }
    }

    pub fn with_root(root: &str) -> Self {
        Self {
            root: Path::new(root).to_path_buf(),
        }
    }

    #[throws]
    pub async fn before(&mut self) -> LocalnetHandle {
        debug!("_____________________");
        debug!("____ BEFORE TEST ____");
        Commander::start_localnet(&self.root).await?
    }

    #[throws]
    pub async fn after(&self, localnet_handle: LocalnetHandle) {
        debug!("____ AFTER TEST ____");
        localnet_handle.stop_and_remove_ledger().await?;
        debug!("_____________________");
    }
}
