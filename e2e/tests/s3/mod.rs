//! S3 API E2E tests.

use std::collections::HashMap;

use cucumber::{given, then, when};
use rusoto_core::{region::Region, RusotoError};
use rusoto_s3::{PutObjectError, PutObjectRequest, S3Client, S3 as _};

use baza::futures_lite::{stream, StreamExt};
use baza_api_s3 as s3;

use super::{sample_file, World, TMP_DIRECTORY};

/// S3 API URL.
const URL: &str = "localhost:9294";

#[when(regex = r"^a file is uploaded to '(\S+)' bucket using '(\S+)' key$")]
async fn file_uploaded(_: &mut World, bucket: String, key: String) {
    put_object(bucket, key, sample_file(), None::<String>).await
}

#[when(regex = "^a symlink is uploaded to '(\\S+)' bucket \
                 using '(\\S+)' key pointing to '(\\S+)'$")]
async fn symlink_is_uploaded(
    _: &mut World,
    bucket: String,
    key: String,
    original: String,
) {
    put_object(bucket, key, &[], Some(original)).await
}

#[then(regex = r"^the file is (?:stored as|accessible via) '(\S+)'$")]
async fn file_is_accessible(_: &mut World, path: String) {
    let stored = async_fs::read(format!("{TMP_DIRECTORY}/{path}"))
        .await
        .expect("file");
    assert!(sample_file() == stored, "Bytes don't match");
}

#[given("keys with leading '/' are considered invalid")]
async fn root_keys_are_invalid(_: &mut World) {
    stream::iter(["/abc", "/abc/d"])
        .then(|key| try_put_object("data", key, &[], None::<String>))
        .for_each(assert_invalid_argument)
        .await;
}

#[given(
    "keys containing '.', '..', '//' path components are considered invalid"
)]
async fn invalid_path_components(_: &mut World) {
    stream::iter([
        "./abc", "abc/.", "abc/./d", "../abc", "abc/..", "abc/../d", "abc//",
        "abc//d",
    ])
    .then(|key| try_put_object("data", key, &[], None::<String>))
    .for_each(assert_invalid_argument)
    .await;
}

fn assert_invalid_argument(res: Result<(), RusotoError<PutObjectError>>) {
    match &res {
        Err(RusotoError::Unknown(resp))
            if resp.body_as_str().contains("InvalidArgument") => {}
        _ => {
            assert!(false, "Expected InvalidArgument error, got: {res:#?}")
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

fn s3_client() -> S3Client {
    S3Client::new(Region::Custom {
        name: "test".to_string(),
        endpoint: format!("http://{URL}"),
    })
}
