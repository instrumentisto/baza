use std::path::PathBuf;

use structopt::StructOpt;

use baza::{tracing::Level, Storage};
use baza_api_s3 as s3;

/// S3 file storage.
#[derive(StructOpt, Debug)]
struct Opts {
    /// Directory where all buckets will be stored.
    #[structopt(short, long, parse(from_os_str), default_value = "files")]
    root: PathBuf,

    /// Logging verbosity level.
    ///
    /// Available values: error, warn, info, debug, trace.
    #[structopt(short, long, parse(try_from_str), default_value = "info")]
    log_level: Level,

    /// Port to run s3 http api on.
    #[structopt(short, long, default_value = "9294")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), s3::RunHttpServerError> {
    let opts = Opts::from_args();

    tracing_subscriber::fmt().with_max_level(opts.log_level).init();

    s3::run_http_server(Storage::new(opts.root), ("0.0.0.0", opts.port)).await
}
