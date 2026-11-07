//! `ApiError`: the single error type of every IPC command.

use serde::Serialize;
use termlite_sftp::ConflictInfo;
use termlite_ssh::{HostKeyInfo, HostKeyMismatch};

/// IPC error. TS: `{ code: "...", detail?: ... }`. Messages must never contain secrets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "code", content = "detail")]
pub enum ApiError {
    /// Vault must be unlocked first.
    #[error("vault locked")]
    VaultLocked,
    /// Wrong master / backup password.
    #[error("wrong password")]
    WrongPassword,
    /// Entity not found.
    #[error("not found")]
    NotFound,
    /// Invalid input (human-readable, no secrets).
    #[error("validation: {0}")]
    Validation(String),
    /// Folder move would create a cycle.
    #[error("cycle")]
    Cycle,
    /// Folder depth > 8.
    #[error("too deep")]
    TooDeep,
    /// Jump chain cycle / > 3 hops.
    #[error("jump cycle")]
    JumpCycle,
    /// Unknown host key — the UI normally gets `SessionEvent::HostKeyPrompt` instead; returned
    /// only by non-interactive paths.
    #[error("host key unknown")]
    HostKeyUnknown(Box<HostKeyInfo>),
    /// Changed host key: red block, NO accept button (only "remove old key in Known hosts").
    #[error("host key changed")]
    HostKeyChanged(Box<HostKeyMismatch>),
    /// Authentication failed.
    #[error("auth failed")]
    AuthFailed {
        /// Methods the server still offers.
        methods_left: Vec<String>,
    },
    /// Network failure (sanitized).
    #[error("network: {0}")]
    Network(String),
    /// Timeout.
    #[error("timeout")]
    Timeout,
    /// SFTP failure (sanitized).
    #[error("sftp: {0}")]
    Sftp(String),
    /// User cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Destination exists.
    #[error("conflict")]
    Conflict(Box<ConflictInfo>),
    /// Feature not available on this platform (e.g. `local_*` on mobile).
    #[error("unsupported")]
    Unsupported,
    /// Anything else (sanitized).
    #[error("internal: {0}")]
    Internal(String),
}

impl ApiError {
    /// Phase 0 stub result.
    pub(crate) fn unimplemented() -> Self {
        Self::Internal("not implemented".into())
    }
}

/// Command result alias.
pub type ApiResult<T> = Result<T, ApiError>;
