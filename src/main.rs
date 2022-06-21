use std::path::PathBuf;

use clap::Parser;

use baza::{tracing::Level, Storage};
use baza_api_s3 as s3;

/// S3 file storage.
#[derive(Parser, Debug)]
struct Args {
    /// Directory where all buckets will be stored.
    #[clap(short, long, parse(from_os_str), default_value = "files")]
    root: PathBuf,

    /// Logging verbosity level.
    ///
    /// Available values: error, warn, info, debug, trace.
    #[clap(short, long, parse(try_from_str), default_value = "info")]
    log_level: Level,

    /// Port to run s3 http api on.
    #[clap(short, long, default_value_t = 9294)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), s3::RunHttpServerError> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .init();

    s3::run_http_server(Storage::new(args.root), ("0.0.0.0", args.port)).await
}
