//! SFTP operations and transfer engine (queue, pipelining, resume).
//!
//! Contract: `docs/contracts/ipc.md` §SFTP + §crates/sftp public API (FROZEN v1). Depends on
//! `crates/ssh` only. Phase 0 skeleton — fallible fns return [`SftpError::Unimplemented`].
// Skeleton-only allowances; the sftp-core agent removes this line when implementing.
#![allow(
    unused_variables,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::unused_self,
    clippy::needless_pass_by_value
)]

pub mod local;
mod model;
pub mod sanitize;
mod transfer;

use ibc_ssh::Connection;

pub use model::{DirListing, Entry, EntryKind};
pub use transfer::{
    ConflictChoice, ConflictId, ConflictInfo, ConflictPolicy, Endpoint, TransferConfig,
    TransferEngine, TransferEvent, TransferId, TransferJob,
};

/// SFTP errors (sanitized messages).
#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    /// No such file.
    #[error("not found")]
    NotFound,
    /// Permission denied.
    #[error("permission denied")]
    PermissionDenied,
    /// Destination exists.
    #[error("already exists")]
    AlreadyExists(ConflictInfo),
    /// Unsafe remote name / path outside the target dir.
    #[error("unsafe path: {0}")]
    UnsafePath(String),
    /// Server status / protocol error.
    #[error("sftp error: {0}")]
    Protocol(String),
    /// Local IO error.
    #[error("io error: {0}")]
    Io(String),
    /// Underlying SSH failure.
    #[error(transparent)]
    Ssh(#[from] ibc_ssh::SshError),
    /// Cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Phase 0 skeleton placeholder.
    #[error("not implemented")]
    Unimplemented,
}

/// An SFTP session over an existing SSH [`Connection`]. All paths are POSIX strings.
#[derive(Debug)]
pub struct Sftp {
    _private: (),
}

impl Sftp {
    /// Opens the `sftp` subsystem and negotiates limits (`limits@openssh.com` if offered).
    pub async fn open(conn: &Connection) -> Result<Self, SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// Lists a directory.
    pub async fn list(&self, path: &str) -> Result<DirListing, SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// `lstat`.
    pub async fn stat(&self, path: &str) -> Result<Entry, SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// `mkdir`.
    pub async fn mkdir(&self, path: &str) -> Result<(), SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// Rename (`posix-rename@openssh.com` if available).
    pub async fn rename(&self, from: &str, to: &str) -> Result<(), SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// Removes a file, or a directory tree when `recursive` (symlinks not followed).
    pub async fn remove(&self, path: &str, recursive: bool) -> Result<(), SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// `setstat` permissions.
    pub async fn chmod(&self, path: &str, mode: u32) -> Result<(), SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// Canonical absolute path (`.` → home directory).
    pub async fn realpath(&self, path: &str) -> Result<String, SftpError> {
        Err(SftpError::Unimplemented)
    }
    /// Closes the subsystem channel.
    pub async fn close(&self) {}
}
