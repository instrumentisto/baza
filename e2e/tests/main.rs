//! E2E (end-to-end) tests of this project.

mod s3;

use std::{collections::HashSet, convert::Infallible};

use cucumber::WorldInit;
use once_cell::sync::Lazy;

use baza::{async_trait, futures_lite::StreamExt as _};

/// Temporary directory for storing files during E2E tests running.
const TMP_DIR: &str = "../.tmp";

#[derive(Debug, Default, WorldInit)]
struct World {
    /// S3 object keys to check for validity.
    keys_to_check: HashSet<String>,
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    clear_tmp_dir().await?;

    World::cucumber()
        .steps(World::collection())
        .repeat_failed()
        .fail_on_skipped()
        .max_concurrent_scenarios(10)
        .run_and_exit("tests")
        .await;

    clear_tmp_dir().await
}

/// Clears the [`TMP_DIR`].
async fn clear_tmp_dir() -> Result<(), String> {
    let is_dir = async_fs::metadata(TMP_DIR)
        .await
        .map(|m| m.is_dir())
        .map_err(|e| format!("Failed to check temporary directory: {e}"))?;

    if !is_dir {
        return Err("TMP_DIR is not a directory".to_string());
    }

    // We can't use `async_fs::remove_dir_all` on `TMP_DIR` directly because
    // doing so breaks Docker bind mount.
    async_fs::read_dir(TMP_DIR)
        .await
        .map_err(|e| format!("Failed to read temporary directory: {e}"))?
        .then(|res| async {
            match res {
                Ok(entry) => async_fs::remove_dir_all(entry.path()).await,
                Err(e) => Err(e),
            }
        })
        .try_collect::<_, _, Vec<_>>()
        .await
        .map(drop)
        .map_err(|e| format!("Failed to remove temporary directory entry: {e}"))
}

/// Reads the `samples/` file and memoizes it.
fn sample_file() -> &'static [u8] {
    static SAMPLE: Lazy<Vec<u8>> = Lazy::new(|| {
        std::fs::read("samples/rms.jpg").expect("Sample file is missing")
    });

    &*SAMPLE
}
