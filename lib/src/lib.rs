use std::{fmt, io, path::PathBuf};

use async_fs::File;
use derive_more::{Display, Error};
use futures::{pin_mut, AsyncWriteExt as _, Stream, StreamExt as _};
use tracerr::Traced;

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
    /// Creates a new [`Storage`].
    ///
    /// If the provided `root` directory doesn't exist yet, tries to create it
    /// recursively.
    ///
    /// # Errors
    ///
    /// - If the provided `root` is an invalid path.
    /// - If the provided `root` is not a directory.
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, io::Error> {
        let path = root.into();
        async_fs::create_dir_all(&path).await?;
        Ok(Storage {
            root: async_fs::canonicalize(path).await?,
        })
    }

    /// Transforms the given [`RelativePath`] into an absolute one.
    fn absolutize(&self, relative: RelativePath) -> PathBuf {
        [self.root.clone(), relative.0].into_iter().collect()
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
        let path = self.absolutize(op.path);
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
        let dest = self.absolutize(op.dest);
        if let Some(dir) = dest.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(tracerr::wrap!())?;
        }

        // TODO: Overwrite already existing link, as `CreateFile` does.
        async_fs::unix::symlink(self.absolutize(op.src), dest)
            .await
            .map_err(tracerr::wrap!())
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
