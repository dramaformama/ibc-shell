//! OS-backed DEK wrapping (`OsKeychain` mode). Desktop/iOS: random KEK in the OS keychain
//! (service `app.ibcshell`, account `vault-kek-v1`). Android: AES-256-GCM key that never
//! leaves Android Keystore (own mobile plugin implements this trait).

use std::sync::Mutex;

use secrecy::{ExposeSecret, SecretBox};
use zeroize::Zeroizing;

use crate::crypto::{open_blob, random_bytes, seal_blob};

const AAD_OS_WRAP: &[u8] = b"tl/dek-wrap-os/v1";

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
    /// Not available on this platform build.
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

fn wrap_with_kek(kek: &[u8; 32], dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError> {
    seal_blob(kek, AAD_OS_WRAP, dek.expose_secret())
        .map_err(|_| KeyStoreError::Platform("wrap failed".into()))
}

fn unwrap_with_kek(kek: &[u8; 32], wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError> {
    let plain = open_blob(kek, AAD_OS_WRAP, wrapped)
        .map_err(|_| KeyStoreError::Platform("unwrap failed".into()))?;
    let arr: [u8; 32] = plain
        .as_slice()
        .try_into()
        .map_err(|_| KeyStoreError::Platform("bad key length".into()))?;
    Ok(SecretBox::new(Box::new(arr)))
}

fn new_kek() -> Result<Zeroizing<[u8; 32]>, KeyStoreError> {
    random_bytes::<32>()
        .map(Zeroizing::new)
        .map_err(|_| KeyStoreError::Platform("rng unavailable".into()))
}

/// In-memory [`KeyStore`] for tests and headless use. The KEK is lost with the process.
#[derive(Default)]
pub struct MemoryKeyStore {
    kek: Mutex<Option<Zeroizing<[u8; 32]>>>,
}

impl std::fmt::Debug for MemoryKeyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryKeyStore")
            .field("kek", &"[REDACTED]")
            .finish()
    }
}

impl MemoryKeyStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyStore for MemoryKeyStore {
    fn wrap_dek(&self, dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError> {
        let mut g = self
            .kek
            .lock()
            .map_err(|_| KeyStoreError::Platform("poisoned".into()))?;
        if g.is_none() {
            *g = Some(new_kek()?);
        }
        let kek = g.as_ref().ok_or(KeyStoreError::NotFound)?;
        wrap_with_kek(kek, dek)
    }
    fn unwrap_dek(&self, wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError> {
        let g = self
            .kek
            .lock()
            .map_err(|_| KeyStoreError::Platform("poisoned".into()))?;
        let kek = g.as_ref().ok_or(KeyStoreError::NotFound)?;
        unwrap_with_kek(kek, wrapped)
    }
    fn reset(&self) -> Result<(), KeyStoreError> {
        *self
            .kek
            .lock()
            .map_err(|_| KeyStoreError::Platform("poisoned".into()))? = None;
        Ok(())
    }
    fn biometric_available(&self) -> bool {
        false
    }
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

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
mod os_impl {
    use super::{new_kek, unwrap_with_kek, wrap_with_kek, KeyStore, KeyStoreError, OsKeyring};
    use secrecy::SecretBox;
    use zeroize::Zeroizing;

    const SERVICE: &str = "app.ibcshell";
    const ACCOUNT: &str = "vault-kek-v1";

    fn entry() -> Result<keyring::Entry, KeyStoreError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(map_err)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn map_err(e: keyring::Error) -> KeyStoreError {
        match e {
            keyring::Error::NoEntry => KeyStoreError::NotFound,
            // Do not forward platform text verbatim: could embed attribute data.
            _ => KeyStoreError::Platform("keychain operation failed".into()),
        }
    }

    fn read_kek(e: &keyring::Entry) -> Result<Zeroizing<[u8; 32]>, KeyStoreError> {
        let raw = Zeroizing::new(e.get_secret().map_err(map_err)?);
        let arr: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| KeyStoreError::Platform("bad kek".into()))?;
        Ok(Zeroizing::new(arr))
    }

    impl KeyStore for OsKeyring {
        fn wrap_dek(&self, dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError> {
            let e = entry()?;
            let kek = match read_kek(&e) {
                Ok(k) => k,
                Err(KeyStoreError::NotFound) => {
                    let k = new_kek()?;
                    e.set_secret(k.as_slice()).map_err(map_err)?;
                    k
                }
                Err(err) => return Err(err),
            };
            wrap_with_kek(&kek, dek)
        }
        fn unwrap_dek(&self, wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError> {
            let kek = read_kek(&entry()?)?;
            unwrap_with_kek(&kek, wrapped)
        }
        fn reset(&self) -> Result<(), KeyStoreError> {
            match entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(map_err(e)),
            }
        }
        fn biometric_available(&self) -> bool {
            false
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
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
