//! S3 API E2E tests.
//!
//! You can enable `standalone` feature and run the server separately:
//!
//! `cargo run -p baza-bin -- -r e2e/tmp`
//! `cargo test -p baza-e2e --features standalone`

use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;
use rusoto_core::{region::Region, RusotoError};
use rusoto_s3::{PutObjectRequest, S3Client, S3 as _};
use tokio::runtime::{self, Runtime};

use baza_api_s3 as s3;

/// S3 API URL.
const URL: &str = "localhost:9294";

/// Temporary root directory for storing test buckets.
const TMP_DIRECTORY: &str = "tmp";

/// E2E runtime.
static RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    let rt = runtime::Builder::new_current_thread().enable_all().build().expect("Failed to build tokio::Runtime");

    let _ = rt.spawn(async {
        let _ = async_fs::remove_dir(TMP_DIRECTORY).await;

        #[cfg(not(feature = "standalone"))]
        s3::run_http_server(baza::Storage::new(TMP_DIRECTORY), URL).await.expect("Failed to run HTTP server")
    });

    Arc::new(rt)
});

#[tokio::test(crate = "executor")]
async fn put_object_works_for_regular_files() {
    let bytes = async_fs::read("samples/rms.jpg").await.expect("rms.jpg");

    let c = S3Client::new(Region::Custom { name: "test".to_string(), endpoint: format!("http://{URL}") });

    c.put_object(PutObjectRequest {
        bucket: "data".to_string(),
        key: "put_object_file/my.jpg".to_string(),
        body: Some(bytes.clone().into()),
        ..PutObjectRequest::default()
    })
    .await
    .expect("put_object");

    let stored_bytes = async_fs::read("tmp/put_object_file/my.jpg").await.expect("rms.jpg");

    assert!(bytes == stored_bytes, "Bytes don't match");
}

#[tokio::test(crate = "executor")]
async fn put_object_works_for_symlinks() {
    let bytes = async_fs::read("samples/rms.jpg").await.expect("rms.jpg");

    let c = S3Client::new(Region::Custom { name: "test".to_string(), endpoint: format!("http://{URL}") });

    c.put_object(PutObjectRequest {
        bucket: "data".to_string(),
        key: "put_object_symlink/my.jpg".to_string(),
        body: Some(bytes.clone().into()),
        ..PutObjectRequest::default()
    })
    .await
    .expect("put_object");

    c.put_object(PutObjectRequest {
        bucket: "links".to_string(),
        key: "put_object_symlink/symlink.jpg".to_string(),
        metadata: Some({
            let mut m = HashMap::new();
            m.insert(s3::SYMLINK_META_KEY.to_string(), "put_object_symlink/my.jpg".to_string());
            m
        }),
        body: Some(vec![].into()),
        ..PutObjectRequest::default()
    })
    .await
    .expect("put_object");

    let stored_bytes = async_fs::read("tmp/put_object_symlink/symlink.jpg").await.expect("symlink.jpg");

    assert!(bytes == stored_bytes, "Bytes don't match");
}

#[tokio::test(crate = "executor")]
async fn put_object_errors_on_invalid_path() {
    let c = S3Client::new(Region::Custom { name: "test".to_string(), endpoint: format!("http://{URL}") });

    let res = c
        .put_object(PutObjectRequest {
            bucket: "data".to_string(),
            key: "/invalid/../".to_string(),
            body: Some(vec![].into()),
            ..PutObjectRequest::default()
        })
        .await;

    match &res {
        Err(RusotoError::Unknown(resp)) if resp.body_as_str().contains("InvalidArgument") => {},
        _ => {
            assert!(false, "Expected InvalidArgument error, got: {res:#?}")
        },
    }
}

/// Compatibility hack to use custom shared `tokio` executor.
pub mod executor {
    pub mod runtime {
        use std::sync::Arc;

        use tokio::runtime::Runtime;

        pub struct Builder;

        impl Builder {
            pub fn new_current_thread() -> Self {
                Self
            }

            pub fn enable_all(self) -> Self {
                self
            }

            pub fn build(self) -> std::io::Result<Arc<Runtime>> {
                Ok(crate::s3::RUNTIME.clone())
            }
        }
    }
}
