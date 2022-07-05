use std::{
    fmt, io,
    path::{Path, PathBuf},
};

use async_fs::File;
use derive_more::{Display, Error};
use futures::{pin_mut, AsyncWriteExt as _, Stream, StreamExt as _};
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
    /// [`Path`] to root directory for storing files.
    ///
    /// [`Path`]: std::path::Path
    root: PathBuf,
}

impl Storage {
    /// Directory to store persistent data.
    const DATA_DIR: &'static str = "data";

    /// Directory to store temporary files.
    const TMP_DIR: &'static str = "tmp";

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

        let data = (&root, Self::DATA_DIR).into_path_buf();
        async_fs::create_dir_all(data)
            .await
            .map_err(tracerr::wrap!())?;

        // Clear tmp directory.
        let tmp = (&root, Self::TMP_DIR).into_path_buf();
        remove_existing_dir(&tmp).await.map_err(tracerr::wrap!())?;
        async_fs::create_dir_all(tmp)
            .await
            .map_err(tracerr::wrap!())?;

        Ok(Storage {
            root: async_fs::canonicalize(root)
                .await
                .map_err(tracerr::wrap!())?,
        })
    }

    /// Transforms the given [`RelativePath`] into an absolute one under the
    /// [`Storage::DATA_DIR`].
    fn absolutize_data_path(&self, relative: RelativePath) -> PathBuf {
        (&self.root, Self::DATA_DIR, relative.0).into_path_buf()
    }

    /// Generates a new random [`PathBuf`] inside [`Storage::TMP_DIR`].
    fn generate_tmp_path(&self) -> PathBuf {
        (&self.root, Self::TMP_DIR, Uuid::new_v4().to_string()).into_path_buf()
    }
}

/// Removes existing directory.
/// Succeeds if directory does not exist already.
///
/// # Errors
///
/// - If [`async_fs::remove_dir_all`] errors with any [`io::ErrorKind`] other
///   than [`io::ErrorKind::NotFound`].
async fn remove_existing_dir(path: impl AsRef<Path>) -> Result<(), io::Error> {
    match async_fs::remove_dir_all(path).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
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
        let path = self.absolutize_data_path(op.path);
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
        let dest = self.absolutize_data_path(op.dest);
        if let Some(dir) = dest.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(tracerr::wrap!())?;
        }
        let src = self.absolutize_data_path(op.src);

        match async_fs::unix::symlink(&src, &dest).await {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(tracerr::new!(e)),
        }

        // We want symlinks to be overwritten atomically, so it's required to
        // do it in 2 steps: first create temporary symlink file and then
        // replace the original file with the temporary one.

        let tmp = self.generate_tmp_path();
        async_fs::unix::symlink(src, &tmp)
            .await
            .map_err(tracerr::wrap!())?;

        async_fs::rename(&tmp, dest).await.map_err(tracerr::wrap!())
    }
}

/// Conversion into [`PathBuf`].
trait IntoPathBuf {
    /// Converts `Self` into [`PathBuf`].
    fn into_path_buf(self) -> PathBuf;
}

impl<A, B> IntoPathBuf for (A, B)
where
    A: AsRef<Path>,
    B: AsRef<Path>,
{
    fn into_path_buf(self) -> PathBuf {
        [self.0.as_ref(), self.1.as_ref()].into_iter().collect()
    }
}

impl<A, B, C> IntoPathBuf for (A, B, C)
where
    A: AsRef<Path>,
    B: AsRef<Path>,
    C: AsRef<Path>,
{
    fn into_path_buf(self) -> PathBuf {
        [self.0.as_ref(), self.1.as_ref(), self.2.as_ref()]
            .into_iter()
            .collect()
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
