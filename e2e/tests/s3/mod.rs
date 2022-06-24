//! S3 HTTP API E2E (end-to-end) tests.

use std::{collections::HashMap, io, mem};

use baza::futures_lite::{stream, StreamExt};
use baza_api_s3 as s3;

use cucumber::{gherkin::Step, then, when};
use rusoto_core::{region::Region, HttpClient, RusotoError};
use rusoto_credential::StaticProvider;
use rusoto_s3::{PutObjectError, PutObjectRequest, S3Client, S3 as _};

use super::{sample_file, World, TMP_DIR};

/// URL of S3 HTTP API to run E2E tests against.
const API_URL: &str = "http://localhost:9294";

#[when(regex = r"^`(\S+)` file is uploaded to `(\S+)` bucket$")]
async fn file_uploaded(_: &mut World, key: String, bucket: String) {
    put_object(bucket, key, sample_file(), None::<String>).await
}

#[when(regex = "^`(\\S+)` symlink is created on `(\\S+)` bucket \
                 pointing to `(\\S+)`$")]
async fn symlink_is_uploaded(
    _: &mut World,
    key: String,
    bucket: String,
    original: String,
) {
    put_object(bucket, key, &[], Some(original)).await
}

#[then(regex = r"^the file is (?:stored as|accessible via) `(\S+)`$")]
async fn file_is_accessible(_: &mut World, path: String) -> io::Result<()> {
    let stored = async_fs::read(format!("{TMP_DIR}/{path}")).await?;

    assert!(sample_file() == stored, "Bytes don`t match");
    Ok(())
}

#[when("trying to upload files with the following keys:")]
async fn keys_table(w: &mut World, step: &Step) {
    w.keys_to_check = step
        .table()
        .expect("No data table present")
        .rows
        .iter()
        .map(|row| row[0].clone())
        .collect();
}

#[then("`InvalidArgument` error is returned")]
async fn invalid_argument_error(w: &mut World) {
    stream::iter(mem::take(&mut w.keys_to_check))
        .then(|key| try_put_object("data", key, &[], None::<String>))
        .for_each(assert_invalid_argument)
        .await;
}

fn assert_invalid_argument(res: Result<(), RusotoError<PutObjectError>>) {
    match &res {
        Err(RusotoError::Unknown(resp))
            if resp.body_as_str().contains("InvalidArgument") => {}
        _ => {
            panic!("Expected InvalidArgument error, got: {res:#?}");
        }
    }
}

async fn put_object(
    bucket: impl ToString,
    key: impl ToString,
    body: &[u8],
    symlink_to: Option<impl ToString>,
) {
    try_put_object(bucket, key, body, symlink_to)
        .await
        .unwrap_or_else(|e| panic!("PutObjectRequest failed: {}", e));
}

async fn try_put_object(
    bucket: impl ToString,
    key: impl ToString,
    body: &[u8],
    symlink_to: Option<impl ToString>,
) -> Result<(), RusotoError<PutObjectError>> {
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        body: Some(body.to_owned().into()),
        metadata: symlink_to.map(|original| {
            let mut meta = HashMap::with_capacity(1);
            meta.insert(s3::SYMLINK_META_KEY.to_string(), original.to_string());
            meta
        }),
        ..PutObjectRequest::default()
    };

    s3_client().put_object(req).await.map(drop)
}

/// Creates a new [`S3Client`] for performing requests to the S3 HTTP API being
/// tested.
fn s3_client() -> S3Client {
    S3Client::new_with(
        HttpClient::new().expect("Failed to initialize Rusoto Http client"),
        StaticProvider::new_minimal("test".to_string(), "test".to_string()),
        Region::Custom {
            name: "test".to_string(),
            endpoint: API_URL.into(),
        },
    )
}
