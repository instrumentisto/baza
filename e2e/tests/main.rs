//! E2E (end-to-end) tests of this project.

mod s3;

use std::{collections::HashSet, convert::Infallible, io};

use cucumber::WorldInit;
use once_cell::sync::Lazy;

use baza::async_trait;

/// Temporary directory for storing files during E2E tests running.
const TMP_DIR: &str = "tmp";

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
    clear_tmp_dir()
        .await
        .map_err(|e| format!("Failed to clear temporary directory: {e}"))?;

    World::cucumber()
        .steps(World::collection())
        .repeat_failed()
        .fail_on_skipped()
        .max_concurrent_scenarios(10)
        .run_and_exit("tests")
        .await;

    clear_tmp_dir()
        .await
        .map_err(|e| format!("Failed to clear temporary directory: {e}"))
}

/// Clears the [`TMP_DIR`].
///
/// # Idempotent
///
/// Succeeds if the directory doesn't exist already.
async fn clear_tmp_dir() -> Result<(), io::Error> {
    if !async_fs::metadata(TMP_DIR)
        .await
        .map(|m| m.is_dir())
        .unwrap_or_default()
    {
        return Ok(());
    }
    async_fs::remove_dir_all(TMP_DIR).await
}

/// Reads the `samples/` file and memoizes it.
fn sample_file() -> &'static [u8] {
    static SAMPLE: Lazy<Vec<u8>> = Lazy::new(|| {
        std::fs::read("samples/rms.jpg").expect("Sample file is missing")
    });

    &*SAMPLE
}
