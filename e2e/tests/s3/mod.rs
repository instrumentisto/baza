//! S3 HTTP API E2E (end-to-end) tests.

use std::{collections::HashMap, io, mem};

use baza::futures::{stream, StreamExt as _};
use baza_api_s3 as s3;

use cucumber::{gherkin::Step, given, then, when};
use rusoto_core::{region::Region, HttpClient, RusotoError};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    GetObjectError, GetObjectRequest, PutObjectError, PutObjectRequest,
    S3Client, S3 as _,
};
use tokio::io::AsyncReadExt as _;

use super::{sample_file, World, DATA_DIR};

/// URL of S3 HTTP API to run E2E tests against.
const API_URL: &str = "http://localhost:9294";

#[given(regex = r"^`(\S+)` was uploaded to `(\S+)` bucket as `(\S+)`$")]
#[when(regex = r"^`(\S+)` is uploaded to `(\S+)` bucket as `(\S+)`$")]
async fn file_uploaded(
    w: &mut World,
    sample: String,
    bucket: String,
    key: String,
) {
    put_object(
        bucket,
        w.unique.filename(key),
        sample_file(sample),
        None::<String>,
    )
    .await
}

#[given(regex = r"^there was nothing uploaded to `(\S+)` bucket as `(\S+)`$")]
async fn nothing_uploaded(_: &mut World) {
    // nothing is uploaded by default
}

#[given(regex = "^`(\\S+)` symlink was created on `(\\S+)` bucket \
                  pointing to `(\\S+)`$")]
#[when(regex = "^`(\\S+)` symlink is created on `(\\S+)` bucket \
                 pointing to `(\\S+)`$")]
async fn symlink_is_uploaded(
    w: &mut World,
    key: String,
    bucket: String,
    original: String,
) {
    put_object(bucket, w.unique.filename(key), &[], Some(original)).await
}

#[then(regex = r"^`(\S+)` is stored as `(\S+)`$")]
async fn file_is_stored(
    w: &mut World,
    sample: String,
    path: String,
) -> io::Result<()> {
    let filename = w.unique.filename(path);
    let stored = async_fs::read(format!("{DATA_DIR}/{filename}")).await?;

    assert!(sample_file(sample) == stored, "Bytes don't match");
    Ok(())
}

#[then(regex = r"^`(\S+)` is accessible via `(\S+)`$")]
async fn file_is_accessible(
    w: &mut World,
    sample: String,
    path: String,
) -> io::Result<()> {
    // We are forced to handle symlinks manually, because Dockerized application
    // has different absolute paths.
    let filename = w.unique.filename(path);
    let src = async_fs::read_link(format!("{DATA_DIR}/{filename}"))
        .await?
        .display()
        .to_string()
        .split(DATA_DIR.trim_matches('.').trim_matches('/'))
        .nth(1)
        .unwrap()
        .trim_matches('/')
        .to_owned();
    file_is_stored(w, sample, src).await
}

#[when("trying to upload files with the following keys:")]
async fn keys_table(w: &mut World, step: &Step) {
    w.keys_to_check = step
        .table()
        .expect("No data table present in the step")
        .rows
        .iter()
        .map(|row| row[0].clone())
        .collect();
}

#[then("`InvalidArgument` error is returned")]
async fn invalid_argument_error(w: &mut World) {
    stream::iter(mem::take(&mut w.keys_to_check))
        .then(|key| try_put_object("data", key, &[], None::<String>))
        .for_each(|res| async move {
            assert_invalid_argument(res);
        })
        .await;
}

#[then(regex = r"^GetObject\(`(\S+)`, `(\S+)`\) returns `(\S+)`$")]
async fn get_object_returns_file(
    w: &mut World,
    bucket: String,
    key: String,
    sample: String,
) {
    let sample = sample_file(sample);
    let file = get_object(bucket, w.unique.filename(key)).await;

    assert_eq!(sample.len(), file.len());
    assert!(sample == file);
}

#[then(regex = r"^GetObject\(`(\S+)`, `(\S+)`\) returns `NoSuchKey` error$")]
async fn get_object_no_such_key(w: &mut World, bucket: String, key: String) {
    let key = w.unique.filename(key);
    let res = try_get_object(bucket, &key).await;

    match &res {
        Err(RusotoError::Service(GetObjectError::NoSuchKey(k))) => {
            assert_eq!(k, &key)
        }
        _ => {
            panic!("Expected NoSuchKey error, got: {res:#?}");
        }
    }
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
        .unwrap_or_else(|e| panic!("`PutObjectRequest` failed: {e}"));
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

async fn get_object(bucket: impl ToString, key: impl ToString) -> Vec<u8> {
    try_get_object(bucket, key)
        .await
        .unwrap_or_else(|e| panic!("`GetObjectRequest` failed: {e}"))
}

async fn try_get_object(
    bucket: impl ToString,
    key: impl ToString,
) -> Result<Vec<u8>, RusotoError<GetObjectError>> {
    let req = GetObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        ..GetObjectRequest::default()
    };

    let resp = s3_client().get_object(req).await?;
    let mut buf = Vec::new();
    resp.body
        .unwrap()
        .into_async_read()
        .read_to_end(&mut buf)
        .await
        .unwrap();

    Ok(buf)
}

/// Creates a new [`S3Client`] for performing requests to the S3 HTTP API being
/// tested.
fn s3_client() -> S3Client {
    S3Client::new_with(
        HttpClient::new().expect("Failed to initialize Rusoto HTTP client"),
        StaticProvider::new_minimal("baza".into(), "baza".into()),
        Region::Custom {
            name: "test".into(),
            endpoint: API_URL.into(),
        },
    )
}
