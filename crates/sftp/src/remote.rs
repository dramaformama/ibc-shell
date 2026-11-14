//! Remote SFTP operations over a `russh-sftp` raw session (pipelining-friendly: every call
//! takes `&self` and requests are multiplexed by id).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::stream::{self, StreamExt};
use ibc_shell_ssh::Connection;
use russh_sftp::client::error::Error as CErr;
use russh_sftp::client::rawsession::Limits;
use russh_sftp::client::{Config, RawSftpSession};
use russh_sftp::protocol::{Attrs, FileAttributes, OpenFlags, Packet, StatusCode};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{DirListing, Entry, EntryKind, SftpError};

const POSIX_RENAME: &str = "posix-rename@openssh.com";
const MAX_DEPTH: usize = 256;

/// An SFTP session over an existing SSH [`Connection`]. All paths are POSIX strings.
pub struct Sftp {
    raw: Arc<RawSftpSession>,
    posix_rename: bool,
    limits: Option<Limits>,
}

impl std::fmt::Debug for Sftp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sftp").finish_non_exhaustive()
    }
}

pub(crate) fn map_err(e: &CErr) -> SftpError {
    match e {
        CErr::Status(s) => match s.status_code {
            StatusCode::NoSuchFile => SftpError::NotFound,
            StatusCode::PermissionDenied => SftpError::PermissionDenied,
            c => {
                let msg: String = s
                    .error_message
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(120)
                    .collect();
                SftpError::Protocol(format!("{c}: {msg}"))
            }
        },
        other => SftpError::Protocol(
            other
                .to_string()
                .chars()
                .filter(|c| !c.is_control())
                .take(160)
                .collect(),
        ),
    }
}

pub(crate) fn is_eof(e: &CErr) -> bool {
    matches!(e, CErr::Status(s) if s.status_code == StatusCode::Eof)
}

pub(crate) fn kind_of(mode: u32) -> EntryKind {
    match mode & 0o170_000 {
        0o040_000 => EntryKind::Dir,
        0o100_000 => EntryKind::File,
        0o120_000 => EntryKind::Symlink,
        _ => EntryKind::Other,
    }
}

fn entry_from(name: String, a: &FileAttributes) -> Entry {
    let mode = a.permissions.unwrap_or(0);
    let kind = kind_of(mode);
    Entry {
        name,
        kind,
        size: if kind == EntryKind::Dir {
            0
        } else {
            a.size.unwrap_or(0)
        },
        mtime: a.mtime.map(|t| i64::from(t) * 1000),
        mode,
        link_target: None,
    }
}

fn ssh_string(out: &mut Vec<u8>, s: &str) {
    let len = u32::try_from(s.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Joins POSIX paths.
pub(crate) fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

impl Sftp {
    /// Opens the `sftp` subsystem and negotiates limits (`limits@openssh.com` if offered).
    pub async fn open(conn: &Connection) -> Result<Self, SftpError> {
        let stream = conn.open_sftp().await?;
        Self::from_stream(stream).await
    }

    /// Runs the SFTP protocol over any byte stream (used by tests).
    pub async fn from_stream<S>(stream: S) -> Result<Self, SftpError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let cfg = Config {
            request_timeout_secs: 60,
            ..Config::default()
        };
        let mut raw = RawSftpSession::new_with_config(stream, cfg);
        let version = raw.init().await.map_err(|e| map_err(&e))?;
        let posix_rename = version
            .extensions
            .get(POSIX_RENAME)
            .is_some_and(|v| v == "1");
        let mut limits = None;
        if version
            .extensions
            .get("limits@openssh.com")
            .is_some_and(|v| v == "1")
        {
            if let Ok(l) = raw.limits().await {
                let l = Limits::from(l);
                raw.set_limits(l);
                limits = Some(l);
            }
        }
        Ok(Self {
            raw: Arc::new(raw),
            posix_rename,
            limits,
        })
    }

    pub(crate) fn raw(&self) -> &Arc<RawSftpSession> {
        &self.raw
    }

    /// Server-advertised `(max_read_len, max_write_len)` if `limits@openssh.com` is offered.
    pub(crate) fn rw_limits(&self) -> (Option<u32>, Option<u32>) {
        let conv = |v: Option<u64>| v.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
        self.limits
            .map_or((None, None), |l| (conv(l.read_len), conv(l.write_len)))
    }

    /// Lists a directory (read in server batches; symlink targets resolved).
    pub async fn list(&self, path: &str) -> Result<DirListing, SftpError> {
        let abs = self.realpath(path).await?;
        let h = self
            .raw
            .opendir(abs.clone())
            .await
            .map_err(|e| map_err(&e))?
            .handle;
        let mut entries = Vec::new();
        let res = loop {
            match self.raw.readdir(h.clone()).await {
                Ok(n) => {
                    for f in n.files {
                        if f.filename == "." || f.filename == ".." {
                            continue;
                        }
                        entries.push(entry_from(f.filename, &f.attrs));
                    }
                    if entries.len() > crate::MAX_DIR_ENTRIES {
                        // A hostile server can answer `readdir` forever.
                        break Err(SftpError::Protocol("directory is too large".into()));
                    }
                }
                Err(e) if is_eof(&e) => break Ok(()),
                Err(e) => break Err(map_err(&e)),
            }
        };
        let _ = self.raw.close(h).await;
        res?;
        for e in entries.iter_mut().filter(|e| e.kind == EntryKind::Symlink) {
            e.link_target = self.readlink(&join(&abs, &e.name)).await.ok();
        }
        Ok(DirListing { path: abs, entries })
    }

    /// `lstat`.
    pub async fn stat(&self, path: &str) -> Result<Entry, SftpError> {
        let Attrs { attrs, .. } = self.raw.lstat(path).await.map_err(|e| map_err(&e))?;
        let name = path.rsplit('/').next().unwrap_or(path).to_owned();
        let mut e = entry_from(name, &attrs);
        if e.kind == EntryKind::Symlink {
            e.link_target = self.readlink(path).await.ok();
        }
        Ok(e)
    }

    /// `lstat`, `None` when missing.
    pub(crate) async fn stat_opt(&self, path: &str) -> Result<Option<Entry>, SftpError> {
        match self.stat(path).await {
            Ok(e) => Ok(Some(e)),
            Err(SftpError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// `mkdir`.
    pub async fn mkdir(&self, path: &str) -> Result<(), SftpError> {
        self.raw
            .mkdir(path, FileAttributes::empty())
            .await
            .map(|_| ())
            .map_err(|e| map_err(&e))
    }

    /// `mkdir -p`.
    pub async fn mkdir_p(&self, path: &str) -> Result<(), SftpError> {
        let mut missing = Vec::new();
        let mut cur = path.trim_end_matches('/').to_owned();
        while !cur.is_empty() {
            match self.stat_opt(&cur).await? {
                Some(e) if e.kind == EntryKind::Dir => break,
                Some(_) => return Err(SftpError::Protocol("not a directory".into())),
                None => {
                    missing.push(cur.clone());
                    cur = cur
                        .rsplit_once('/')
                        .map_or(String::new(), |(p, _)| p.to_owned());
                }
            }
        }
        for p in missing.iter().rev() {
            if let Err(e) = self.mkdir(p).await {
                // Lost a race with another creator: fine if it is a directory now.
                if !matches!(self.stat_opt(p).await?, Some(x) if x.kind == EntryKind::Dir) {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Rename (`posix-rename@openssh.com` if available).
    pub async fn rename(&self, from: &str, to: &str) -> Result<(), SftpError> {
        if self.posix_rename {
            let mut data = Vec::new();
            ssh_string(&mut data, from);
            ssh_string(&mut data, to);
            return match self.raw.extended(POSIX_RENAME, data).await {
                Ok(Packet::Status(s)) if s.status_code == StatusCode::Ok => Ok(()),
                Ok(Packet::Status(s)) => Err(map_err(&CErr::Status(s))),
                Ok(_) => Err(SftpError::Protocol("unexpected reply".into())),
                Err(e) => Err(map_err(&e)),
            };
        }
        self.raw
            .rename(from, to)
            .await
            .map(|_| ())
            .map_err(|e| map_err(&e))
    }

    /// Rename that replaces an existing destination even without the posix extension.
    pub(crate) async fn rename_over(&self, from: &str, to: &str) -> Result<(), SftpError> {
        if !self.posix_rename && self.stat_opt(to).await?.is_some() {
            self.raw.remove(to).await.map_err(|e| map_err(&e))?;
        }
        self.rename(from, to).await
    }

    /// Removes a file, or a directory tree when `recursive` (symlinks not followed).
    pub async fn remove(&self, path: &str, recursive: bool) -> Result<(), SftpError> {
        let e = self.stat(path).await?;
        if e.kind != EntryKind::Dir {
            return self
                .raw
                .remove(path)
                .await
                .map(|_| ())
                .map_err(|e| map_err(&e));
        }
        if recursive {
            self.remove_tree(path.to_owned(), 0).await
        } else {
            self.raw
                .rmdir(path)
                .await
                .map(|_| ())
                .map_err(|e| map_err(&e))
        }
    }

    fn remove_tree(
        &self,
        dir: String,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<(), SftpError>> + Send + '_>> {
        Box::pin(async move {
            if depth > MAX_DEPTH {
                return Err(SftpError::Protocol("directory tree too deep".into()));
            }
            let listing = self.list(&dir).await?;
            let (dirs, files): (Vec<_>, Vec<_>) = listing
                .entries
                .into_iter()
                .partition(|e| e.kind == EntryKind::Dir);
            let dir_ref = &dir;
            let results: Vec<Result<(), SftpError>> = stream::iter(files)
                .map(|e| async move {
                    self.raw
                        .remove(join(dir_ref, &e.name))
                        .await
                        .map(|_| ())
                        .map_err(|e| map_err(&e))
                })
                .buffer_unordered(8)
                .collect()
                .await;
            results.into_iter().collect::<Result<(), _>>()?;
            for d in dirs {
                self.remove_tree(join(&dir, &d.name), depth + 1).await?;
            }
            self.raw
                .rmdir(dir)
                .await
                .map(|_| ())
                .map_err(|e| map_err(&e))
        })
    }

    /// `setstat` permissions.
    pub async fn chmod(&self, path: &str, mode: u32) -> Result<(), SftpError> {
        let a = FileAttributes {
            permissions: Some(mode & 0o7777),
            ..FileAttributes::empty()
        };
        self.raw
            .setstat(path, a)
            .await
            .map(|_| ())
            .map_err(|e| map_err(&e))
    }

    /// Sets access/modification time (unix millis).
    pub(crate) async fn set_mtime(&self, path: &str, mtime_ms: i64) -> Result<(), SftpError> {
        let secs = u32::try_from((mtime_ms / 1000).max(0)).unwrap_or(u32::MAX);
        let a = FileAttributes {
            atime: Some(secs),
            mtime: Some(secs),
            ..FileAttributes::empty()
        };
        self.raw
            .setstat(path, a)
            .await
            .map(|_| ())
            .map_err(|e| map_err(&e))
    }

    /// Canonical absolute path (`.` → home directory).
    pub async fn realpath(&self, path: &str) -> Result<String, SftpError> {
        let n = self.raw.realpath(path).await.map_err(|e| map_err(&e))?;
        n.files
            .into_iter()
            .next()
            .map(|f| f.filename)
            .ok_or_else(|| SftpError::Protocol("empty realpath reply".into()))
    }

    /// Symlink target.
    pub async fn readlink(&self, path: &str) -> Result<String, SftpError> {
        let n = self.raw.readlink(path).await.map_err(|e| map_err(&e))?;
        n.files
            .into_iter()
            .next()
            .map(|f| f.filename)
            .ok_or_else(|| SftpError::Protocol("empty readlink reply".into()))
    }

    /// Creates `path` as a symlink to `target`.
    pub async fn symlink(&self, path: &str, target: &str) -> Result<(), SftpError> {
        // OpenSSH order: (target, link path), swapped vs the draft spec.
        self.raw
            .symlink(target, path)
            .await
            .map(|_| ())
            .map_err(|e| map_err(&e))
    }

    /// Closes the subsystem channel.
    #[allow(clippy::unused_async)] // frozen API
    pub async fn close(&self) {
        let _ = self.raw.close_session();
    }

    /// Opens a remote file; returns the handle.
    pub(crate) async fn open_file(
        &self,
        path: &str,
        flags: OpenFlags,
    ) -> Result<String, SftpError> {
        self.raw
            .open(path, flags, FileAttributes::empty())
            .await
            .map(|h| h.handle)
            .map_err(|e| map_err(&e))
    }
}
