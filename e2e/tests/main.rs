//! E2E (end-to-end) tests of project.

use std::{convert::Infallible, io};

use cucumber::{runner, WorldInit};
use once_cell::sync::Lazy;

use baza::async_trait;

mod s3;

/// Temporary root directory for storing files.
const TMP_DIRECTORY: &str = "tmp";

#[derive(Debug, WorldInit)]
struct World;

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

#[tokio::main]
async fn main() {
    let _ = clear_tmp_dir().await;

    World::cucumber()
        .with_runner(runner::Basic::default())
        .steps(World::collection())
        .repeat_failed()
        .fail_on_skipped()
        .max_concurrent_scenarios(10)
        .run_and_exit("tests")
        .await;

    clear_tmp_dir()
        .await
        .expect("Failed to clear tmp directory")
}

async fn clear_tmp_dir() -> Result<(), io::Error> {
    async_fs::remove_dir_all(TMP_DIRECTORY).await
}

fn sample_file() -> &'static [u8] {
    static SAMPLE: Lazy<Vec<u8>> = Lazy::new(|| {
        std::fs::read("samples/rms.jpg").expect("Sample file is missing")
    });

    &*SAMPLE
}
