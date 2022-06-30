//! E2E (end-to-end) tests of this project.

mod s3;

use std::{collections::HashSet, convert::Infallible, fs};

use cucumber::WorldInit;
use once_cell::sync::Lazy;

use baza::{async_trait, futures::TryStreamExt as _};

/// Path to the directory where files are stored during E2E tests running.
const DATA_DIR: &str = "../.cache/data";

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
    clear_data_dir().await?;

    World::cucumber()
        .steps(World::collection())
        .repeat_failed()
        .fail_on_skipped()
        .max_concurrent_scenarios(10)
        .run_and_exit("tests")
        .await;

    clear_data_dir().await
}

/// Clears contents of the [`DATA_DIR`].
async fn clear_data_dir() -> Result<(), String> {
    async_fs::metadata(DATA_DIR)
        .await
        .map_err(|e| format!("Cannot stat `{DATA_DIR}` dir: {e}"))?
        .is_dir()
        .then(|| ())
        .ok_or_else(|| format!("`{DATA_DIR}` is not a dir"))?;

    // We cannot use `async_fs::remove_dir_all` on `DATA_DIR` directly, because
    // doing so breaks the running S3 server (it looses its directory).
    async_fs::read_dir(DATA_DIR)
        .await
        .map_err(|e| format!("Cannot read `{DATA_DIR}` dir: {e}"))?
        .map_err(|e| format!("Cannot read entry from `{DATA_DIR}` dir: {e}"))
        .try_for_each(|entry| async move {
            let path = entry.path();
            async_fs::remove_dir_all(path.as_path()).await.map_err(|e| {
                format!("Cannot remove `{}` dir: {e}", path.display())
            })
        })
        .await
}

/// Reads the `samples/` file and memoizes it.
fn sample_file() -> &'static [u8] {
    static SAMPLE: Lazy<Vec<u8>> = Lazy::new(|| {
        fs::read("samples/rms.jpg").expect("Sample file is missing")
    });

    &*SAMPLE
}
