//! Vault: DEK/KEK, secret encryption, OS keychain, backup format.
//!
//! Contract: `docs/contracts/security.md` §Vault + §Vault public API (FROZEN v1).

mod backup;
mod crypto;
mod idle;
pub mod keys;
pub mod keystore;
mod storage;
#[cfg(test)]
mod tests;
mod types;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ibc_shell_store::{SecretId, SecretRow, Store, StoreError};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crypto::{open_blob, seal_blob, KdfParams};
pub use idle::DEFAULT_TIMEOUT as DEFAULT_IDLE_TIMEOUT;
pub use keystore::{KeyStore, KeyStoreError, MemoryKeyStore, OsKeyring};
pub use storage::VaultStorage;
pub use types::{ImportSummary, Result, VaultError, VaultMode, VaultState};

const META_VERSION: &str = "version";
const META_MODE: &str = "mode";
const META_KDF: &str = "kdf";
const META_WRAP_PW: &str = "dek_wrap_pw";
const META_WRAP_OS: &str = "dek_wrap_os";
const META_CHECK: &str = "check";
const AAD_WRAP: &[u8] = b"tl/dek-wrap/v1";
const AAD_CHECK: &[u8] = b"tl/check/v1";
const AAD_SECRET: &[u8] = b"tl/secret/v1";
const CHECK_PLAIN: &[u8] = b"tl-ok";
const MIN_MASTER_PASSWORD_CHARS: usize = 8;

/// The vault. Holds the DEK in a `SecretBox<[u8; 32]>` while unlocked; zeroized on lock/drop.
/// `Send + Sync`; all methods are blocking (Argon2) — call via `spawn_blocking`.
pub struct Vault {
    storage: Arc<dyn VaultStorage>,
    keystore: Arc<dyn KeyStore>,
    dek: Mutex<Option<SecretBox<[u8; 32]>>>,
    /// Backend idle auto-lock deadline (ADR-007).
    idle: idle::Idle,
    /// (m KiB, t, p) used for new wraps; tests lower it.
    cost: (u32, u32, u32),
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("keys", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// AAD binding a ciphertext to its row: `"tl/secret/v1" || id`.
pub(crate) fn secret_aad(id: SecretId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(AAD_SECRET.len() + 16);
    aad.extend_from_slice(AAD_SECRET);
    aad.extend_from_slice(id.0.as_bytes());
    aad
}

fn dek_from_slice(b: &[u8]) -> Result<SecretBox<[u8; 32]>> {
    let arr: [u8; 32] = b.try_into().map_err(|_| VaultError::Corrupt)?;
    Ok(SecretBox::new(Box::new(arr)))
}

impl Vault {
    /// Creates a locked vault over `store` (vault never opens SQLite itself).
    #[must_use]
    pub fn new(store: Arc<Store>, keystore: Arc<dyn KeyStore>) -> Self {
        Self::with_storage(store, keystore)
    }

    /// Creates a vault over any [`VaultStorage`].
    #[must_use]
    pub fn with_storage(storage: Arc<dyn VaultStorage>, keystore: Arc<dyn KeyStore>) -> Self {
        Self {
            storage,
            keystore,
            dek: Mutex::new(None),
            idle: idle::Idle::default(),
            cost: (65_536, 3, 1),
        }
    }

    /// Sets the backend idle auto-lock (`None` = never; ADR-007) and restarts its clock.
    pub fn set_idle_timeout(&self, timeout: Option<Duration>) {
        self.idle.set_timeout(timeout);
    }

    /// Records user activity (extends the idle deadline). No effect on a locked vault.
    pub fn touch(&self) {
        self.idle.touch_at(Instant::now());
    }

    /// Locks (zeroizes the DEK) if the idle deadline passed. `true` = it locked just now.
    pub fn lock_if_idle(&self) -> bool {
        let mut g = self.dek_guard();
        if g.is_some() && self.idle.expired_at(Instant::now()) {
            *g = None;
            return true;
        }
        false
    }

    /// Lowers the Argon2 cost for new wraps (low-RAM mobile, tests). Recorded in meta.
    #[must_use]
    pub fn with_kdf_cost(mut self, m_kib: u32, t: u32, p: u32) -> Self {
        self.cost = (m_kib, t, p);
        self
    }

    fn dek_guard(&self) -> MutexGuard<'_, Option<SecretBox<[u8; 32]>>> {
        self.dek.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// DEK guard after enforcing the idle deadline (an expired DEK is zeroized here).
    fn live_guard(&self) -> MutexGuard<'_, Option<SecretBox<[u8; 32]>>> {
        let mut g = self.dek_guard();
        if g.is_some() && self.idle.expired_at(Instant::now()) {
            *g = None;
        }
        g
    }

    /// Runs `f` with the DEK; `Locked` when no DEK is loaded or the idle deadline passed.
    pub(crate) fn with_dek<R>(&self, f: impl FnOnce(&[u8; 32]) -> Result<R>) -> Result<R> {
        let g = self.live_guard();
        let dek = g.as_ref().ok_or(VaultError::Locked)?;
        f(dek.expose_secret())
    }

    pub(crate) fn storage(&self) -> &dyn VaultStorage {
        &*self.storage
    }

    /// Test hook: is a DEK currently held in memory?
    #[cfg(test)]
    pub(crate) fn dek_loaded(&self) -> bool {
        self.dek_guard().is_some()
    }

    fn initialized(&self) -> Result<bool> {
        Ok(self.storage.meta_get(META_VERSION)?.is_some())
    }

    fn check_version(&self) -> Result<()> {
        match self.storage.meta_get(META_VERSION)? {
            None => Err(VaultError::Uninitialized),
            Some(v) if v.as_slice() == [0, 1] => Ok(()),
            Some(v) if v.len() == 2 => Err(VaultError::UnsupportedVersion(u16::from_be_bytes([
                v[0], v[1],
            ]))),
            Some(_) => Err(VaultError::Corrupt),
        }
    }

    fn meta_required(&self, k: &str) -> Result<Vec<u8>> {
        self.storage.meta_get(k)?.ok_or(VaultError::Corrupt)
    }

    /// Current lock state.
    pub fn state(&self) -> Result<VaultState> {
        if self.live_guard().is_some() {
            return Ok(VaultState::Unlocked);
        }
        Ok(if self.initialized()? {
            VaultState::Locked
        } else {
            VaultState::Uninitialized
        })
    }

    /// Configured mode; `None` when uninitialized.
    pub fn mode(&self) -> Result<Option<VaultMode>> {
        if !self.initialized()? {
            return Ok(None);
        }
        match self.meta_required(META_MODE)?.as_slice() {
            b"os" => Ok(Some(VaultMode::OsKeychain)),
            b"pw" => Ok(Some(VaultMode::MasterPassword)),
            _ => Err(VaultError::Corrupt),
        }
    }

    /// Whether unlock can show a biometric prompt.
    #[must_use]
    pub fn biometric_available(&self) -> bool {
        self.keystore.biometric_available()
    }

    fn wrap_with_password(&self, dek: &[u8; 32], password: &SecretString) -> Result<()> {
        if password.expose_secret().chars().count() < MIN_MASTER_PASSWORD_CHARS {
            return Err(VaultError::WeakPassword);
        }
        let params = KdfParams::with_cost(self.cost.0, self.cost.1, self.cost.2)?;
        let kek = params.derive(password)?;
        let wrap = seal_blob(&kek, AAD_WRAP, dek)?;
        self.storage.meta_set(META_KDF, &params.to_cbor()?)?;
        self.storage.meta_set(META_WRAP_PW, &wrap)?;
        Ok(())
    }

    fn unwrap_with_password(&self, password: &SecretString) -> Result<SecretBox<[u8; 32]>> {
        let params = KdfParams::from_cbor(&self.meta_required(META_KDF)?)?;
        let wrap = self.meta_required(META_WRAP_PW)?;
        let kek = params.derive(password)?;
        let dek = open_blob(&kek, AAD_WRAP, &wrap).map_err(|_| VaultError::WrongPassword)?;
        dek_from_slice(&dek)
    }

    fn verify_check(&self, dek: &[u8; 32]) -> Result<()> {
        let plain = open_blob(dek, AAD_CHECK, &self.meta_required(META_CHECK)?)?;
        if bool::from(plain.as_slice().ct_eq(CHECK_PLAIN)) {
            Ok(())
        } else {
            Err(VaultError::Corrupt)
        }
    }

    fn install(&self, dek: SecretBox<[u8; 32]>) {
        let mut g = self.dek_guard();
        self.idle.touch_at(Instant::now()); // a fresh unlock starts a fresh idle period
        *g = Some(dek);
    }

    /// Creates DEK + wraps it. `MasterPassword` requires `password`. Leaves the vault unlocked.
    pub fn init(&self, mode: VaultMode, password: Option<&SecretString>) -> Result<()> {
        if self.initialized()? {
            return Err(VaultError::AlreadyInitialized);
        }
        let password = match (mode, password) {
            (VaultMode::MasterPassword, None) => return Err(VaultError::WeakPassword),
            (VaultMode::MasterPassword, Some(p)) => Some(p),
            (VaultMode::OsKeychain, _) => None,
        };
        let dek = dek_from_slice(&crypto::random_bytes::<32>()?)?;
        let key = dek.expose_secret();
        if let Some(p) = password {
            self.wrap_with_password(key, p)?;
            self.storage.meta_set(META_MODE, b"pw")?;
        } else {
            let wrapped = self.keystore.wrap_dek(&dek)?;
            self.storage.meta_set(META_WRAP_OS, &wrapped)?;
            self.storage.meta_set(META_MODE, b"os")?;
        }
        self.storage
            .meta_set(META_CHECK, &seal_blob(key, AAD_CHECK, CHECK_PLAIN)?)?;
        // Version last: a half-written init still reads as Uninitialized.
        self.storage.meta_set(META_VERSION, &[0, 1])?;
        self.install(dek);
        Ok(())
    }

    /// Unlocks with the master password.
    pub fn unlock_with_password(&self, password: &SecretString) -> Result<()> {
        self.check_version()?;
        if self.mode()? != Some(VaultMode::MasterPassword) {
            return Err(VaultError::WrongPassword);
        }
        let dek = self.unwrap_with_password(password)?;
        self.verify_check(dek.expose_secret())?;
        self.install(dek);
        Ok(())
    }

    /// Unlocks via the [`KeyStore`] (keychain / biometric).
    pub fn unlock_with_keystore(&self) -> Result<()> {
        self.check_version()?;
        let wrapped = self
            .storage
            .meta_get(META_WRAP_OS)?
            .ok_or(KeyStoreError::NotFound)?;
        let dek = self.keystore.unwrap_dek(&wrapped)?;
        self.verify_check(dek.expose_secret())?;
        self.install(dek);
        Ok(())
    }

    /// Zeroizes the DEK. Idempotent.
    pub fn lock(&self) {
        // SecretBox zeroizes its contents on drop.
        *self.dek_guard() = None;
    }

    /// Rewraps the DEK under a new password; secrets untouched.
    pub fn change_password(&self, old: &SecretString, new: &SecretString) -> Result<()> {
        self.check_version()?;
        if self.mode()? != Some(VaultMode::MasterPassword) {
            return Err(VaultError::WrongPassword);
        }
        let dek = self.unwrap_with_password(old)?;
        self.verify_check(dek.expose_secret())?;
        self.wrap_with_password(dek.expose_secret(), new)
    }

    /// Encrypts and stores a secret, returns its new id.
    pub fn put_secret(&self, plaintext: &[u8]) -> Result<SecretId> {
        let id = SecretId::new_v7();
        self.put_secret_with_id(id, plaintext)?;
        Ok(id)
    }

    pub(crate) fn put_secret_with_id(&self, id: SecretId, plaintext: &[u8]) -> Result<()> {
        let (nonce, ct) = self.with_dek(|dek| crypto::seal(dek, &secret_aad(id), plaintext))?;
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
        self.storage.secret_put(SecretRow {
            id,
            nonce: nonce.to_vec(),
            ciphertext: ct,
            created_at,
        })?;
        Ok(())
    }

    /// Decrypts a secret.
    pub fn get_secret(&self, id: SecretId) -> Result<Zeroizing<Vec<u8>>> {
        // Locked is reported before NotFound so a locked vault leaks nothing.
        self.with_dek(|_| Ok(()))?;
        let row = self.storage.secret_get(id)?.ok_or(StoreError::NotFound)?;
        self.with_dek(|dek| crypto::open(dek, &secret_aad(id), &row.nonce, &row.ciphertext))
    }

    /// Closure-based access: the plaintext never leaves as an owned value.
    pub fn with_secret<R>(&self, id: SecretId, f: impl FnOnce(&[u8]) -> R) -> Result<R> {
        let s = self.get_secret(id)?;
        Ok(f(&s))
    }

    /// Deletes a secret row.
    pub fn delete_secret(&self, id: SecretId) -> Result<()> {
        self.with_dek(|_| Ok(()))?;
        self.storage.secret_delete(id)?;
        Ok(())
    }

    /// Writes a `.tlvault` backup (requires unlocked vault; `password` ≥ 10 chars).
    pub fn export_backup(&self, path: &Path, password: &SecretString) -> Result<()> {
        backup::export(self, path, password)
    }

    /// Reads a `.tlvault` backup and merges it (requires unlocked vault).
    pub fn import_backup(&self, path: &Path, password: &SecretString) -> Result<ImportSummary> {
        backup::import(self, path, password)
    }
}
