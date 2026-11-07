//! One end of a transfer: local FS or remote SFTP, with the few operations the engine needs.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use russh_sftp::protocol::OpenFlags;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, BufWriter};

use crate::remote::{join, map_err};
use crate::{local, Endpoint, Entry, EntryKind, Sftp, SftpError};

/// Resolved endpoint.
#[derive(Clone)]
pub(crate) enum Side {
    Local,
    Remote(Arc<Sftp>),
}

impl From<&Endpoint> for Side {
    fn from(e: &Endpoint) -> Self {
        match e {
            Endpoint::Local => Self::Local,
            Endpoint::Remote(s) => Self::Remote(s.clone()),
        }
    }
}

/// Readable file.
pub(crate) enum Source {
    Local(File),
    Remote(Arc<Sftp>, String),
}

/// Writable file positioned at the resume offset.
pub(crate) enum Sink {
    Local(BufWriter<File>),
    Remote(Arc<Sftp>, String),
}

impl Source {
    pub(crate) async fn close(self) {
        if let Self::Remote(s, h) = self {
            let _ = s.raw().close(h).await;
        }
    }
}

impl Sink {
    pub(crate) async fn finish(self) -> Result<(), SftpError> {
        match self {
            Self::Local(mut w) => {
                tokio::io::AsyncWriteExt::flush(&mut w).await?;
                Ok(())
            }
            Self::Remote(s, h) => s.raw().close(h).await.map(|_| ()).map_err(|e| map_err(&e)),
        }
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, SftpError> + Send + 'static,
) -> Result<T, SftpError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| SftpError::Io(e.to_string()))?
}

impl Side {
    pub(crate) fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    pub(crate) fn join(&self, dir: &str, name: &str) -> String {
        match self {
            Self::Local => PathBuf::from(dir).join(name).to_string_lossy().into_owned(),
            Self::Remote(_) => join(dir, name),
        }
    }

    /// `lstat`; `None` when missing.
    pub(crate) async fn lstat(&self, path: &str) -> Result<Option<Entry>, SftpError> {
        match self {
            Self::Local => {
                let p = PathBuf::from(path);
                match blocking(move || local::lstat(&p)).await {
                    Ok(e) => Ok(Some(e)),
                    Err(SftpError::NotFound) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Self::Remote(s) => s.stat_opt(path).await,
        }
    }

    pub(crate) async fn list(&self, path: &str) -> Result<Vec<Entry>, SftpError> {
        match self {
            Self::Local => {
                let p = PathBuf::from(path);
                blocking(move || local::list(&p).map(|l| l.entries)).await
            }
            Self::Remote(s) => s.list(path).await.map(|l| l.entries),
        }
    }

    pub(crate) async fn mkdir_p(&self, path: &str) -> Result<(), SftpError> {
        match self {
            Self::Local => tokio::fs::create_dir_all(path).await.map_err(Into::into),
            Self::Remote(s) => s.mkdir_p(path).await,
        }
    }

    /// Atomic rename replacing the destination.
    pub(crate) async fn rename_over(&self, from: &str, to: &str) -> Result<(), SftpError> {
        match self {
            Self::Local => tokio::fs::rename(from, to).await.map_err(Into::into),
            Self::Remote(s) => s.rename_over(from, to).await,
        }
    }

    pub(crate) async fn remove_file(&self, path: &str) -> Result<(), SftpError> {
        match self {
            Self::Local => tokio::fs::remove_file(path).await.map_err(Into::into),
            Self::Remote(s) => s.remove(path, false).await,
        }
    }

    /// Applies mtime (unix ms) and permission bits; best effort on `mode`.
    pub(crate) async fn set_meta(
        &self,
        path: &str,
        mtime: Option<i64>,
        mode: Option<u32>,
    ) -> Result<(), SftpError> {
        match self {
            Self::Local => {
                let p = PathBuf::from(path);
                blocking(move || set_local_meta(&p, mtime, mode)).await
            }
            Self::Remote(s) => {
                if let Some(m) = mtime {
                    s.set_mtime(path, m).await?;
                }
                if let Some(m) = mode {
                    let _ = s.chmod(path, m & 0o777).await;
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn open_read(&self, path: &str) -> Result<Source, SftpError> {
        match self {
            Self::Local => Ok(Source::Local(File::open(path).await?)),
            Self::Remote(s) => {
                let h = s.open_file(path, OpenFlags::READ).await?;
                Ok(Source::Remote(s.clone(), h))
            }
        }
    }

    /// Opens for writing at `start` (0 truncates, otherwise existing bytes are kept).
    pub(crate) async fn open_write(&self, path: &str, start: u64) -> Result<Sink, SftpError> {
        match self {
            Self::Local => {
                let mut f = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(start == 0)
                    .open(path)
                    .await?;
                if start > 0 {
                    f.seek(std::io::SeekFrom::Start(start)).await?;
                }
                Ok(Sink::Local(BufWriter::with_capacity(1 << 20, f)))
            }
            Self::Remote(s) => {
                let mut fl = OpenFlags::WRITE | OpenFlags::CREATE;
                if start == 0 {
                    fl |= OpenFlags::TRUNCATE;
                }
                Ok(Sink::Remote(s.clone(), s.open_file(path, fl).await?))
            }
        }
    }

    /// Final size check helper: size of a file, error if missing.
    pub(crate) async fn size_of(&self, path: &str) -> Result<u64, SftpError> {
        match self.lstat(path).await? {
            Some(e) if e.kind == EntryKind::File => Ok(e.size),
            Some(_) => Err(SftpError::Protocol("not a regular file".into())),
            None => Err(SftpError::NotFound),
        }
    }

    pub(crate) async fn truncate(&self, path: &str, size: u64) -> Result<(), SftpError> {
        match self {
            Self::Local => {
                let f = tokio::fs::OpenOptions::new().write(true).open(path).await?;
                f.set_len(size).await.map_err(Into::into)
            }
            // Remote: a resume never leaves extra bytes beyond `size` unless the source
            // shrank; the size check afterwards reports it.
            Self::Remote(_) => Ok(()),
        }
    }
}

fn set_local_meta(p: &Path, mtime: Option<i64>, mode: Option<u32>) -> Result<(), SftpError> {
    if let Some(ms) = mtime {
        let t = UNIX_EPOCH + Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0));
        let f = std::fs::OpenOptions::new().write(true).open(p)?;
        f.set_modified(t)?;
    }
    #[cfg(unix)]
    if let Some(m) = mode {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(m & 0o777))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}
