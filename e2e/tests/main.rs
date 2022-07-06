//! E2E (end-to-end) tests of this project.

mod s3;

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fs,
};

use cucumber::WorldInit;
use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, thread_rng, Rng as _};

use baza::{async_trait, futures::TryStreamExt as _};

/// Path to the directory where files are stored during E2E tests running.
const DATA_DIR: &str = "../.cache/baza/data";

#[derive(Debug, Default, WorldInit)]
struct World {
    /// Random string of the concrete scenario run, to enrich its data with, for
    /// avoiding possible collisions with other running scenarios.
    unique: Unique,

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
        .then_some(())
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

/// Reads the `samples/` directory, memoizes sample files, and returns the
/// requested `sample`.
fn sample_file(sample: impl AsRef<str>) -> &'static [u8] {
    static SAMPLES: Lazy<HashMap<String, Vec<u8>>> = Lazy::new(|| {
        fs::read_dir("samples")
            .expect("Samples directory is missing")
            .map(|e| {
                let name = e
                    .expect("Failed to read sample directory entry")
                    .file_name()
                    .into_string()
                    .expect("Sample filename is not a valid String");

                let content = fs::read(format!("samples/{}", name))
                    .expect("Failed to read a sample file");

                (name, content)
            })
            .collect()
    });

    let sample = sample.as_ref();
    SAMPLES
        .get(sample)
        .unwrap_or_else(|| panic!("sample `{sample}` doesn't exist"))
}

/// Random string to enrich E2E scenario data with, for avoiding collisions.
#[derive(Clone, Debug)]
struct Unique(String);

impl Default for Unique {
    fn default() -> Self {
        Self(
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(|n| char::from(n).to_ascii_lowercase())
                .collect(),
        )
    }
}

impl Unique {
    /// Forms an [`Unique`] filename out with the provided `prefix`.
    #[must_use]
    fn filename(&self, prefix: impl AsRef<str>) -> String {
        format!("{}-{}", prefix.as_ref(), self.0)
    }
}
