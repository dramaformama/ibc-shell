//! `ApiError`: the single error type of every IPC command.

use ibc_shell_sftp::ConflictInfo;
use ibc_shell_ssh::{HostKeyInfo, HostKeyMismatch};
use serde::Serialize;

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

/// Command result alias.
pub type ApiResult<T> = Result<T, ApiError>;

impl From<ibc_shell_ssh::SshError> for ApiError {
    fn from(e: ibc_shell_ssh::SshError) -> Self {
        use ibc_shell_ssh::SshError as E;
        match e {
            E::HostKeyChanged(m) => Self::HostKeyChanged(m),
            E::AuthFailed { methods_left } => Self::AuthFailed { methods_left },
            E::Network(s) | E::Negotiation(s) | E::Protocol(s) => Self::Network(s),
            E::Timeout => Self::Timeout,
            E::Cancelled => Self::Cancelled,
            E::InvalidChain => Self::JumpCycle,
            E::KnownHosts(_) => Self::Internal("known hosts backend failure".into()),
            E::Unimplemented | E::Unsupported => Self::Unsupported,
        }
    }
}

impl From<ibc_shell_sftp::SftpError> for ApiError {
    fn from(e: ibc_shell_sftp::SftpError) -> Self {
        use ibc_shell_sftp::SftpError as E;
        match e {
            E::NotFound => Self::NotFound,
            E::PermissionDenied => Self::Sftp("permission denied".into()),
            E::AlreadyExists(info) => Self::Conflict(Box::new(info)),
            E::UnsafePath(s) => Self::Validation(s),
            E::Protocol(s) | E::Io(s) => Self::Sftp(s),
            E::Ssh(e) => e.into(),
            E::Cancelled => Self::Cancelled,
            E::Unimplemented => Self::Unsupported,
        }
    }
}

impl From<ibc_shell_store::StoreError> for ApiError {
    fn from(e: ibc_shell_store::StoreError) -> Self {
        use ibc_shell_store::StoreError as E;
        match e {
            E::NotFound => Self::NotFound,
            E::Validation(s) | E::Conflict(s) => Self::Validation(s),
            E::Cycle => Self::Cycle,
            E::TooDeep => Self::TooDeep,
            E::JumpCycle => Self::JumpCycle,
            E::Db(_) => Self::Internal("database error".into()),
            E::Unimplemented => Self::Unsupported,
        }
    }
}

impl From<ibc_shell_vault::VaultError> for ApiError {
    fn from(e: ibc_shell_vault::VaultError) -> Self {
        use ibc_shell_vault::VaultError as E;
        match e {
            E::Locked => Self::VaultLocked,
            E::WrongPassword => Self::WrongPassword,
            E::WeakPassword => Self::Validation("weak password".into()),
            E::Store(s) => s.into(),
            E::Unimplemented => Self::Unsupported,
            other => Self::Internal(other.to_string()),
        }
    }
}
