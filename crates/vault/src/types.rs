//! Public vault types: lock state, mode, import summary, errors.

use ibc_store::StoreError;
use serde::{Deserialize, Serialize};

use crate::keystore::KeyStoreError;

/// Lock state. IPC: `VaultStatus.state`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum VaultState {
    /// No `vault_meta.version` yet.
    Uninitialized,
    /// Initialized, DEK not in memory.
    Locked,
    /// DEK in memory.
    Unlocked,
}

/// How the DEK is wrapped. IPC: `VaultStatus.mode`, `vault_init(mode)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum VaultMode {
    /// KEK in OS keychain / Android Keystore (`dek_wrap_os`).
    OsKeychain,
    /// Argon2id(master password) (`dek_wrap_pw`), optional OS cache for biometrics.
    MasterPassword,
}

/// Counts returned by `vault_import`. IPC: `ImportSummary`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportSummary {
    /// Folders added.
    pub folders: u32,
    /// Hosts added.
    pub hosts: u32,
    /// Identities added (with secrets).
    pub identities: u32,
    /// Tags added.
    pub tags: u32,
    /// Snippets added.
    pub snippets: u32,
    /// Forwards added.
    pub forwards: u32,
    /// Known hosts added.
    pub known_hosts: u32,
    /// Rows skipped because the id already existed.
    pub skipped: u32,
}

/// Vault errors. Never contain secret material; `Debug` is derived only because no variant
/// carries secrets.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// Operation needs an unlocked vault.
    #[error("vault locked")]
    Locked,
    /// `init` on an initialized vault.
    #[error("vault already initialized")]
    AlreadyInitialized,
    /// Operation on an uninitialized vault.
    #[error("vault not initialized")]
    Uninitialized,
    /// Wrong master / backup password (constant-time compare after Argon2).
    #[error("wrong password")]
    WrongPassword,
    /// Password policy (backup: ≥ 10 chars).
    #[error("weak password")]
    WeakPassword,
    /// AEAD failure / tampered data / bad backup header.
    #[error("decryption failed")]
    Corrupt,
    /// Unsupported backup / meta version.
    #[error("unsupported version {0}")]
    UnsupportedVersion(u16),
    /// Store failure.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Keystore failure.
    #[error(transparent)]
    KeyStore(#[from] KeyStoreError),
    /// File IO (sanitized message).
    #[error("io error: {0}")]
    Io(String),
    /// Reserved; never returned by this implementation.
    #[error("not implemented")]
    Unimplemented,
}

/// Result alias.
pub type Result<T, E = VaultError> = std::result::Result<T, E>;
