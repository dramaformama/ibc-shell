//! Vault: DEK/KEK, secret encryption, OS keychain, backup format.
//!
//! Contract: `docs/contracts/security.md` §Vault + §Vault public API (FROZEN v1).
//! Phase 0 skeleton — every fallible method returns [`VaultError::Unimplemented`].
// Skeleton-only allowances; the vault agent removes this line when implementing.
#![allow(unused_variables, clippy::unused_self, clippy::needless_pass_by_value)]

pub mod keystore;

use std::path::Path;
use std::sync::Arc;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use termlite_store::{SecretId, Store, StoreError};
use zeroize::Zeroizing;

pub use keystore::{KeyStore, KeyStoreError, OsKeyring};

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
    /// Phase 0 skeleton placeholder.
    #[error("not implemented")]
    Unimplemented,
}

/// Result alias.
pub type Result<T, E = VaultError> = std::result::Result<T, E>;

/// The vault. Holds the DEK in a `SecretBox<[u8; 32]>` while unlocked; zeroized on lock/drop.
/// `Send + Sync`; all methods are blocking (Argon2) — call via `spawn_blocking`.
pub struct Vault {
    store: Arc<Store>,
    keystore: Arc<dyn KeyStore>,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault").finish_non_exhaustive()
    }
}

impl Vault {
    /// Creates a locked vault over `store` (vault never opens SQLite itself).
    #[must_use]
    pub fn new(store: Arc<Store>, keystore: Arc<dyn KeyStore>) -> Self {
        Self { store, keystore }
    }
    /// Current lock state.
    pub fn state(&self) -> Result<VaultState> {
        Err(VaultError::Unimplemented)
    }
    /// Configured mode; `None` when uninitialized.
    pub fn mode(&self) -> Result<Option<VaultMode>> {
        Err(VaultError::Unimplemented)
    }
    /// Whether unlock can show a biometric prompt.
    #[must_use]
    pub fn biometric_available(&self) -> bool {
        self.keystore.biometric_available()
    }
    /// Creates DEK + wraps it. `MasterPassword` requires `password`. Leaves the vault unlocked.
    pub fn init(&self, mode: VaultMode, password: Option<&SecretString>) -> Result<()> {
        Err(VaultError::Unimplemented)
    }
    /// Unlocks with the master password.
    pub fn unlock_with_password(&self, password: &SecretString) -> Result<()> {
        Err(VaultError::Unimplemented)
    }
    /// Unlocks via the [`KeyStore`] (keychain / biometric).
    pub fn unlock_with_keystore(&self) -> Result<()> {
        Err(VaultError::Unimplemented)
    }
    /// Zeroizes the DEK. Idempotent.
    pub fn lock(&self) {}
    /// Rewraps the DEK under a new password; secrets untouched.
    pub fn change_password(&self, old: &SecretString, new: &SecretString) -> Result<()> {
        Err(VaultError::Unimplemented)
    }
    /// Encrypts and stores a secret, returns its new id.
    pub fn put_secret(&self, plaintext: &[u8]) -> Result<SecretId> {
        Err(VaultError::Unimplemented)
    }
    /// Decrypts a secret.
    pub fn get_secret(&self, id: SecretId) -> Result<Zeroizing<Vec<u8>>> {
        Err(VaultError::Unimplemented)
    }
    /// Deletes a secret row.
    pub fn delete_secret(&self, id: SecretId) -> Result<()> {
        Err(VaultError::Unimplemented)
    }
    /// Writes a `.tlvault` backup (requires unlocked vault; `password` ≥ 10 chars).
    pub fn export_backup(&self, path: &Path, password: &SecretString) -> Result<()> {
        let _ = &self.store;
        Err(VaultError::Unimplemented)
    }
    /// Reads a `.tlvault` backup and merges it (requires unlocked vault).
    pub fn import_backup(&self, path: &Path, password: &SecretString) -> Result<ImportSummary> {
        Err(VaultError::Unimplemented)
    }
}
