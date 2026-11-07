//! SFTP operations and transfer engine (queue, pipelining, resume).
//!
//! Contract: `docs/contracts/ipc.md` §SFTP + §crates/sftp public API (FROZEN v1). Depends on
//! `crates/ssh` only.

mod copy;
mod engine;
mod job;
pub mod local;
mod model;
mod remote;
pub mod sanitize;
mod scan;
mod side;
mod transfer;

pub use model::{DirListing, Entry, EntryKind};
pub use engine::TransferEngine;
pub use remote::Sftp;
pub use transfer::{
    ConflictChoice, ConflictId, ConflictInfo, ConflictPolicy, Endpoint, TransferConfig,
    TransferEvent, TransferId, TransferJob,
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
    /// Not implemented.
    #[error("not implemented")]
    Unimplemented,
}

impl From<std::io::Error> for SftpError {
    fn from(e: std::io::Error) -> Self {
        local::io_err(&e)
    }
}
