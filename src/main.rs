use std::path::PathBuf;

use baza::{tracing::Level, Storage};
use baza_api_s3 as s3;

use secrecy::SecretString;

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = <CliOpts as clap::Parser>::parse();

    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .init();

    let storage = Storage::new(args.root).await.map_err(|e| {
        format!("Failed to initialize `Storage`: {e}: {}", e.trace())
    })?;

    s3::run_http_server(
        storage,
        ("0.0.0.0", args.port),
        args.access_key,
        args.secret_key,
    )
    .await
    .map_err(|e| format!("Failed to run S3 HTTP server: {e}"))
}

/// CLI options.
#[derive(Debug, clap::Parser)]
struct CliOpts {
    /// Directory where all buckets will be stored.
    #[clap(short, long, parse(from_os_str), default_value = "/var/lib/baza")]
    root: PathBuf,

    /// Logging verbosity level.
    ///
    /// Available values: `error`, `warn`, `info`, `debug`, `trace`.
    #[clap(short, long, parse(try_from_str), default_value = "info")]
    log_level: Level,

    /// Port to run S3 HTTP API on.
    #[clap(short, long, default_value_t = 9294)]
    port: u16,

    /// S3 API access key.
    #[clap(
        long,
        parse(try_from_str),
        env = "BAZA_ACCESS_KEY",
        default_value = "baza"
    )]
    access_key: SecretString,

    /// S3 API secret key.
    #[clap(
        long,
        parse(try_from_str),
        env = "BAZA_SECRET_KEY",
        default_value = "baza"
    )]
    secret_key: SecretString,
}
