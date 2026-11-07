//! OS-backed DEK wrapping (`OsKeychain` mode). Desktop/iOS: random KEK in the OS keychain
//! (service `app.termlite`, account `vault-kek-v1`). Android: AES-256-GCM key that never
//! leaves Android Keystore (own mobile plugin implements this trait).

use secrecy::SecretBox;

/// Errors from a [`KeyStore`]. Never contain key material.
#[derive(Debug, thiserror::Error)]
pub enum KeyStoreError {
    /// No key stored (first run, or the user deleted it).
    #[error("key not found")]
    NotFound,
    /// User cancelled the biometric / presence prompt.
    #[error("user cancelled")]
    Cancelled,
    /// Platform API failure (sanitized message).
    #[error("keystore error: {0}")]
    Platform(String),
    /// Phase 0 skeleton placeholder.
    #[error("not implemented")]
    Unimplemented,
}

/// Wraps / unwraps the 32-byte DEK with a platform-held key. Implementations must be cheap
/// to call from a blocking thread; the vault never calls them on the async runtime.
pub trait KeyStore: Send + Sync {
    /// Wraps `dek`, creating the platform key on first use. Returns an opaque blob stored in
    /// `vault_meta.dek_wrap_os`.
    fn wrap_dek(&self, dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError>;
    /// Unwraps a blob produced by [`KeyStore::wrap_dek`]. May show a biometric prompt.
    fn unwrap_dek(&self, wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError>;
    /// Deletes the platform key (vault reset / mode switch).
    fn reset(&self) -> Result<(), KeyStoreError>;
    /// Whether unwrapping is gated by biometrics on this device.
    fn biometric_available(&self) -> bool;
}

/// Desktop/iOS implementation over the `keyring` crate (KEK + XChaCha20-Poly1305 wrap).
#[derive(Debug, Default)]
pub struct OsKeyring {
    _private: (),
}

impl OsKeyring {
    /// Creates the keyring-backed store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyStore for OsKeyring {
    fn wrap_dek(&self, _dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError> {
        Err(KeyStoreError::Unimplemented)
    }
    fn unwrap_dek(&self, _wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError> {
        Err(KeyStoreError::Unimplemented)
    }
    fn reset(&self) -> Result<(), KeyStoreError> {
        Err(KeyStoreError::Unimplemented)
    }
    fn biometric_available(&self) -> bool {
        false
    }
}
