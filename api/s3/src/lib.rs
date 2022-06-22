//! S3 HTTP API Baza implementation.

use std::{
    convert::Infallible,
    fmt, io,
    net::{TcpListener, ToSocketAddrs},
};

use hyper::{server::Server, service::make_service_fn};
use s3_server::{
    dto,
    errors::{S3Error, S3ErrorCode, S3StorageResult},
    S3Service, S3Storage,
};

use baza::{
    async_trait,
    derive_more::{Display, Error, From},
    futures_lite::future,
    tracing::{self, info},
    CreateFile, Exec, RelativePath, Symlink,
};

/// [`dto::PutObjectRequest::metadata`] key where [`Symlink::original`] is
/// expected to be provided.
pub const SYMLINK_META_KEY: &str = "symlink-to";

/// Runs s3 http server.
///
/// # Errors
///
/// See [`RunHttpServerError`].
pub async fn run_http_server<S, A>(
    storage: S,
    addr: A,
) -> Result<(), RunHttpServerError>
where
    A: ToSocketAddrs,
    S3<S>: S3Storage + Send + Sync + 'static,
{
    let service = S3Service::new(S3(storage)).into_shared();
    let listener = TcpListener::bind(addr)?;
    let make_service = make_service_fn(move |_| {
        future::ready(Ok::<_, Infallible>(service.clone()))
    });

    info!("Starting S3 HTTP Server");
    Ok(Server::from_tcp(listener)?.serve(make_service).await?)
}

/// Errors of [`run_http_server`] fn.
#[derive(Debug, Display, Error, From)]
pub enum RunHttpServerError {
    /// Failed to bind address.
    #[display(fmt = "Failed to bind address: {}", _0)]
    BindAddress(io::Error),

    /// [`hyper`] server failure.
    #[display(fmt = "Hyper server failed: {}", _0)]
    Hyper(hyper::Error),
}

/// Local wrapper for implementing foreign traits on foreign types.
#[derive(Clone, Debug)]
pub struct S3<T>(T);

#[async_trait]
impl<S, E1, E2> S3Storage for S3<S>
where
    S: Exec<CreateFile<dto::ByteStream>, Err = E1>
        + Exec<Symlink, Err = E2>
        + fmt::Debug
        + Send
        + Sync
        + 'static,
    E1: fmt::Display,
    E2: fmt::Display,
{
    async fn complete_multipart_upload(
        &self,
        _: dto::CompleteMultipartUploadRequest,
    ) -> S3StorageResult<
        dto::CompleteMultipartUploadOutput,
        dto::CompleteMultipartUploadError,
    > {
        unimplemented!()
    }

    async fn copy_object(
        &self,
        _: dto::CopyObjectRequest,
    ) -> S3StorageResult<dto::CopyObjectOutput, dto::CopyObjectError> {
        unimplemented!()
    }

    async fn create_multipart_upload(
        &self,
        _: dto::CreateMultipartUploadRequest,
    ) -> S3StorageResult<
        dto::CreateMultipartUploadOutput,
        dto::CreateMultipartUploadError,
    > {
        unimplemented!()
    }

    async fn create_bucket(
        &self,
        _: dto::CreateBucketRequest,
    ) -> S3StorageResult<dto::CreateBucketOutput, dto::CreateBucketError> {
        unimplemented!()
    }

    async fn delete_bucket(
        &self,
        _: dto::DeleteBucketRequest,
    ) -> S3StorageResult<dto::DeleteBucketOutput, dto::DeleteBucketError> {
        unimplemented!()
    }

    async fn delete_object(
        &self,
        _: dto::DeleteObjectRequest,
    ) -> S3StorageResult<dto::DeleteObjectOutput, dto::DeleteObjectError> {
        unimplemented!()
    }

    async fn delete_objects(
        &self,
        _: dto::DeleteObjectsRequest,
    ) -> S3StorageResult<dto::DeleteObjectsOutput, dto::DeleteObjectsError>
    {
        unimplemented!()
    }

    async fn get_bucket_location(
        &self,
        _: dto::GetBucketLocationRequest,
    ) -> S3StorageResult<
        dto::GetBucketLocationOutput,
        dto::GetBucketLocationError,
    > {
        unimplemented!()
    }

    async fn get_object(
        &self,
        _: dto::GetObjectRequest,
    ) -> S3StorageResult<dto::GetObjectOutput, dto::GetObjectError> {
        unimplemented!()
    }

    async fn head_bucket(
        &self,
        _: dto::HeadBucketRequest,
    ) -> S3StorageResult<dto::HeadBucketOutput, dto::HeadBucketError> {
        unimplemented!()
    }

    async fn head_object(
        &self,
        _: dto::HeadObjectRequest,
    ) -> S3StorageResult<dto::HeadObjectOutput, dto::HeadObjectError> {
        unimplemented!()
    }

    async fn list_buckets(
        &self,
        _: dto::ListBucketsRequest,
    ) -> S3StorageResult<dto::ListBucketsOutput, dto::ListBucketsError> {
        unimplemented!()
    }

    async fn list_objects(
        &self,
        _: dto::ListObjectsRequest,
    ) -> S3StorageResult<dto::ListObjectsOutput, dto::ListObjectsError> {
        unimplemented!()
    }

    async fn list_objects_v2(
        &self,
        _: dto::ListObjectsV2Request,
    ) -> S3StorageResult<dto::ListObjectsV2Output, dto::ListObjectsV2Error>
    {
        unimplemented!()
    }

    #[tracing::instrument(
       skip_all,
       fields(bucket = input.bucket.as_str(), key = input.key.as_str()),
    )]
    async fn put_object(
        &self,
        input: dto::PutObjectRequest,
    ) -> S3StorageResult<dto::PutObjectOutput, dto::PutObjectError> {
        let path = parse_s3_path(input.bucket, input.key)?;

        if let Some(original) = input
            .metadata
            .and_then(|mut meta| meta.remove(SYMLINK_META_KEY))
        {
            let op = Symlink {
                original: parse_relative_path(SYMLINK_META_KEY, original)?,
                link: path,
            };

            self.0
                .exec(op)
                .await
                .map_err(|e| internal_error("Symlink operation failed", e))?;
        } else {
            let op = CreateFile {
                path,
                bytes: input.body.unwrap_or_else(|| vec![].into()),
            };

            self.0.exec(op).await.map_err(|e| {
                internal_error("CreateFile operation failed", e)
            })?;
        };

        info!("OK");
        Ok(dto::PutObjectOutput::default())
    }

    async fn upload_part(
        &self,
        _: dto::UploadPartRequest,
    ) -> S3StorageResult<dto::UploadPartOutput, dto::UploadPartError> {
        unimplemented!()
    }
}

/// Parses `bucket` and `key` into a single [`RelativePath`].
fn parse_s3_path(bucket: String, key: String) -> Result<RelativePath, S3Error> {
    Ok(parse_relative_path("bucket", bucket)?
        .join(parse_relative_path("key", key)?))
}

/// Parses a string into [`RelativePath`].
fn parse_relative_path(attr: &str, s: String) -> Result<RelativePath, S3Error> {
    s.try_into().map_err(|e| {
        S3Error::new(
            S3ErrorCode::InvalidArgument,
            format!("Invalid {attr}: {e}"),
        )
    })
}

/// Constructs internal [`S3Error`].
fn internal_error<E: fmt::Display>(msg: &str, e: E) -> S3Error {
    S3Error::new(S3ErrorCode::InternalError, format!("{msg}: {e}"))
}
