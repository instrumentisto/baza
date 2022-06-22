pub use async_trait::async_trait;
pub use derive_more;
pub use futures_lite;
pub use tracing;

use std::{fmt, io, path::PathBuf};

use async_fs::{unix::symlink, File};
use derive_more::{Display, Error};
use futures_lite::{AsyncWriteExt as _, Stream, StreamExt as _};
use tracerr::Traced;

/// Execute a filesystem operation.
#[async_trait]
pub trait Exec<Op> {
    type Ok;
    type Err;

    async fn exec(&self, op: Op) -> Result<Self::Ok, Self::Err>;
}

/// File storage.
#[derive(Clone, Debug)]
pub struct Storage {
    /// Root directory for storing files.
    root: PathBuf,
}

impl Storage {
    /// Creates a new [`Storage`].
    ///
    /// If the provided `root` directory does not exist yet, tries to create it
    /// along with all its parent directories.
    ///
    /// # Errors
    ///
    /// - If `root` is an invalid path
    /// - If `root` is not a directory
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, io::Error> {
        let path = root.into();
        async_fs::create_dir_all(&path).await?;

        Ok(Storage {
            root: async_fs::canonicalize(path).await?,
        })
    }

    /// Transforms a [`RelativePath`] into an absolute one.
    fn absolutize(&self, relative: RelativePath) -> PathBuf {
        [self.root.clone(), relative.0].into_iter().collect()
    }
}

/// Creates a new file.
#[derive(Clone, Debug)]
pub struct CreateFile<S> {
    /// [`RelativePath`] of the file to create.
    pub path: RelativePath,

    /// [`Stream`] of file bytes.
    pub bytes: S,
}

#[async_trait]
impl<S, B> Exec<CreateFile<S>> for Storage
where
    S: Stream<Item = Result<B, io::Error>>
        + Unpin
        + Send
        + fmt::Debug
        + 'static,
    B: AsRef<[u8]> + Send + Sync,
{
    type Ok = ();
    type Err = Traced<io::Error>;

    #[tracing::instrument(level = "debug", err(Debug))]
    async fn exec(&self, mut op: CreateFile<S>) -> Result<Self::Ok, Self::Err> {
        let path = self.absolutize(op.path);

        if let Some(dir) = path.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(|e| tracerr::new!(e))?;
        }

        let mut f = File::create(path).await.map_err(|e| tracerr::new!(e))?;

        while let Some(res) = op.bytes.next().await {
            f.write_all(res.map_err(|e| tracerr::new!(e))?.as_ref())
                .await
                .map_err(|e| tracerr::new!(e))?;
        }

        f.flush().await.map_err(|e| tracerr::new!(e))
    }
}

/// Create a symlink.
#[derive(Debug, Clone)]
pub struct Symlink {
    /// [`RelativePath`] of the original file.
    pub original: RelativePath,

    /// [`RelativePath`] of the symlink itself.
    pub link: RelativePath,
}

#[async_trait]
impl Exec<Symlink> for Storage {
    type Ok = ();
    type Err = Traced<io::Error>;

    #[tracing::instrument(level = "debug", err(Debug))]
    async fn exec(&self, op: Symlink) -> Result<Self::Ok, Self::Err> {
        let link = self.absolutize(op.link);

        if let Some(dir) = link.parent() {
            async_fs::create_dir_all(dir)
                .await
                .map_err(|e| tracerr::new!(e))?;
        }

        symlink(self.absolutize(op.original), link)
            .await
            .map_err(|e| tracerr::new!(e))
    }
}

/// Filesystem path relative to the configured root folder.
///
/// The following [`PathBuf`] components are forbidden:
/// - Root (leading '/')
/// - CurDir ('.')
/// - ParentDir ('..')
/// - empty component ('//')
#[derive(Clone, Debug)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    /// Joins another [`RelativePath`]s to this one.
    pub fn join(mut self, other: RelativePath) -> Self {
        self.0.push(other.0);
        self
    }
}

impl TryFrom<String> for RelativePath {
    type Error = RelativePathError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with('/')
            | s.split('/').any(|c| matches!(c, "" | "." | ".."))
        {
            tracing::warn!("Invalid RelativePath");
            Err(RelativePathError(()))
        } else {
            Ok(RelativePath(s.into()))
        }
    }
}

/// Error of parsing [`RelativePath`] from a [`String`].
#[derive(Debug, Display, Error)]
#[display(fmt = "'RelativePath' should not:
                  - start with the Root('/') component
                  - contain empty, CurDir('.') or ParentDir('..') components")]
pub struct RelativePathError(#[error(not(source))] ());

#[cfg(test)]
mod test {
    use super::*;

    mod relative_path_spec {
        use super::*;

        fn case(s: &str) -> Result<RelativePath, RelativePathError> {
            RelativePath::try_from(s.to_string())
        }

        #[test]
        fn cant_start_with_root_component() {
            assert!(case("/my/path").is_err());
        }

        #[test]
        fn cant_contain_empty_component() {
            assert!(case("//my/path").is_err());
            assert!(case("my//path").is_err());
            assert!(case("my/path//").is_err());
        }

        #[test]
        fn cant_contain_cur_dir_component() {
            assert!(case("/./my/path").is_err());
            assert!(case("./my/path").is_err());
            assert!(case("my/./path").is_err());
            assert!(case("my/path/.").is_err());
            assert!(case("my/path/./").is_err());
        }

        #[test]
        fn cant_contain_parent_dir_component() {
            assert!(case("/../my/path").is_err());
            assert!(case("../my/path").is_err());
            assert!(case("my/../path").is_err());
            assert!(case("my/path/..").is_err());
            assert!(case("my/path/../").is_err());
        }
    }
}
