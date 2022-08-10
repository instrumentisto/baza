use std::{
    fmt, io,
    path::{Path, PathBuf},
    pin::Pin,
    task,
};

use async_fs::File;
use derive_more::{Display, Error};
use futures::{pin_mut, AsyncRead, AsyncWriteExt as _, Stream, StreamExt as _};
use tracerr::Traced;
use uuid::Uuid;

pub use async_trait::async_trait;
pub use derive_more;
pub use futures;
pub use tracing;

/// Execution of a filesystem operation.
#[async_trait]
pub trait Exec<Operation> {
    type Ok;
    type Err;

    /// Executes the provided `operation`.
    async fn exec(&self, operation: Operation) -> Result<Self::Ok, Self::Err>;
}

/// Storage backed by files and directories.
#[derive(Clone, Debug)]
pub struct Storage {
    /// Absolute [`Path`] to the directory to persist data in.
    data_dir: PathBuf,

    /// Absolute [`Path`] to the directory to use for storing temporary data.
    ///
    /// [System's temporary directory][0] is not used, because it may be located
    /// on another device or has another filesystem, thus doesn't guarantee
    /// atomicity of [copy](async_fs::copy) and [rename](async_fs::rename)
    /// operations, which is vital for how this directory is used.
    ///
    /// [0]: https://en.wikipedia.org/wiki/Temporary_folder
    tmp_dir: PathBuf,
}

impl Storage {
    /// Creates a new [`Storage`].
    ///
    /// If the provided `root` directory doesn't exist yet, tries to create it
    /// recursively.
    ///
    /// # Errors
    ///
    /// - If the provided `root` is an invalid path.
    /// - If the provided `root` is not a directory.
    pub async fn new(
        root: impl Into<PathBuf>,
    ) -> Result<Self, Traced<io::Error>> {
        let root = root.into();

        let data = root.join("data");
        async_fs::create_dir_all(&data)
            .await
            .map_err(tracerr::wrap!())?;

        let tmp = root.join("tmp");
        remove_existing_dir(&tmp).await.map_err(tracerr::wrap!())?;
        async_fs::create_dir_all(&tmp)
            .await
            .map_err(tracerr::wrap!())?;

        Ok(Self {
            data_dir: async_fs::canonicalize(data)
                .await
                .map_err(tracerr::wrap!())?,
            tmp_dir: async_fs::canonicalize(tmp)
                .await
                .map_err(tracerr::wrap!())?,
        })
    }
}

/// Removes the existing `dir`ectory.
///
/// # Idempotent
///
/// Succeeds if the `dir`ectory doesn't exist already.
///
/// # Errors
///
/// If [`async_fs::remove_dir_all()`] errors with anything other than
/// [`io::ErrorKind::NotFound`].
async fn remove_existing_dir(dir: impl AsRef<Path>) -> io::Result<()> {
    async_fs::remove_dir_all(dir).await.or_else(|e| {
        (e.kind() == io::ErrorKind::NotFound).then_some(()).ok_or(e)
    })
}

/// Operation of a new file creation.
#[derive(Clone, Debug)]
pub struct CreateFile<Bytes> {
    /// [`RelativePath`] of the file to be created.
    pub path: RelativePath,

    /// [`Stream`] of file bytes.
    pub bytes: Bytes,
}

#[async_trait]
impl<S, Bytes> Exec<CreateFile<S>> for Storage
where
    S: Stream<Item = Result<Bytes, io::Error>> + fmt::Debug + Send + 'static,
    Bytes: AsRef<[u8]> + Send + Sync,
{
    type Ok = ();
    type Err = Traced<io::Error>;

    #[tracing::instrument(level = "debug", err(Debug))]
    async fn exec(&self, op: CreateFile<S>) -> Result<Self::Ok, Self::Err> {
        let path = self.data_dir.join(op.path);
        if let Some(dir) = path.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(tracerr::wrap!())?;
        }

        let mut f = File::create(path).await.map_err(tracerr::wrap!())?;

        let bytes = op.bytes;
        pin_mut!(bytes);
        while let Some(res) = bytes.next().await {
            f.write_all(res.map_err(tracerr::wrap!())?.as_ref())
                .await
                .map_err(tracerr::wrap!())?;
        }
        f.flush().await.map_err(tracerr::wrap!())
    }
}

/// Operation of a symlink creation.
#[derive(Debug, Clone)]
pub struct CreateSymlink {
    /// [`RelativePath`] of the original source file.
    pub src: RelativePath,

    /// [`RelativePath`] of the symlink itself.
    pub dest: RelativePath,
}

#[async_trait]
impl Exec<CreateSymlink> for Storage {
    type Ok = ();
    type Err = Traced<io::Error>;

    #[tracing::instrument(level = "debug", err(Debug))]
    async fn exec(&self, op: CreateSymlink) -> Result<Self::Ok, Self::Err> {
        let dest = self.data_dir.join(op.dest);
        if let Some(dir) = dest.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(tracerr::wrap!())?;
        }
        let src = self.data_dir.join(op.src);

        match async_fs::unix::symlink(&src, &dest).await {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(tracerr::new!(e)),
        }

        // We want symlinks to be overwritten atomically, so it's required to
        // do this in 2 steps:
        // 1. create temporary symlink file;
        // 2. replace the original file with the temporary one.
        let tmp = self.tmp_dir.join(Uuid::new_v4().to_string());
        async_fs::unix::symlink(src, &tmp)
            .await
            .map_err(tracerr::wrap!())?;
        async_fs::rename(&tmp, dest).await.map_err(tracerr::wrap!())
    }
}

/// Operation for getting an existing file.
#[derive(Debug, Clone)]
pub struct GetFile {
    /// [`RelativePath`] of the file.
    pub path: RelativePath,
}

#[async_trait]
impl Exec<GetFile> for Storage {
    type Ok = Option<ReadOnlyFile>;
    type Err = Traced<io::Error>;

    #[tracing::instrument(level = "debug", err(Debug))]
    async fn exec(&self, op: GetFile) -> Result<Self::Ok, Self::Err> {
        let path = self.data_dir.join(op.path);

        match File::open(&path).await {
            Ok(f) => Ok(Some(ReadOnlyFile(f))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(tracerr::new!(e)),
        }
    }
}

/// Read-only [`File`].
pub struct ReadOnlyFile(File);

impl AsyncRead for ReadOnlyFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut [u8],
    ) -> task::Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

/// Filesystem path relative to the configured root folder.
///
/// # Format
///
/// The following [`PathBuf`] components are forbidden:
/// - root (leading `/`)
/// - current directory (`.`)
/// - parent directory (`..`)
/// - empty component (`//`)
#[derive(Clone, Debug)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    /// Joins another [`RelativePath`]s to this one.
    #[must_use]
    pub fn join(mut self, other: RelativePath) -> Self {
        self.0.push(other.0);
        self
    }
}

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl TryFrom<String> for RelativePath {
    type Error = InvalidRelativePathError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with('/')
            || s.split('/').any(|c| matches!(c, "" | "." | ".."))
        {
            tracing::warn!("Invalid RelativePath");
            Err(InvalidRelativePathError)
        } else {
            Ok(RelativePath(s.into()))
        }
    }
}

/// Error of parsing [`RelativePath`] from a [`String`].
#[derive(Debug, Display, Error)]
#[display(fmt = "Invalid `RelativePath` format")]
pub struct InvalidRelativePathError;

#[cfg(test)]
mod relative_path_spec {
    use super::{InvalidRelativePathError, RelativePath};

    fn case(s: &str) -> Result<RelativePath, InvalidRelativePathError> {
        RelativePath::try_from(s.to_string())
    }

    #[test]
    fn cannot_start_with_root_component() {
        assert!(case("/my/path").is_err());
    }

    #[test]
    fn cannot_contain_empty_component() {
        assert!(case("//my/path").is_err());
        assert!(case("my//path").is_err());
        assert!(case("my/path//").is_err());
    }

    #[test]
    fn cannot_contain_cur_dir_component() {
        assert!(case("/./my/path").is_err());
        assert!(case("./my/path").is_err());
        assert!(case("my/./path").is_err());
        assert!(case("my/path/.").is_err());
        assert!(case("my/path/./").is_err());
    }

    #[test]
    fn cannot_contain_parent_dir_component() {
        assert!(case("/../my/path").is_err());
        assert!(case("../my/path").is_err());
        assert!(case("my/../path").is_err());
        assert!(case("my/path/..").is_err());
        assert!(case("my/path/../").is_err());
    }
}
