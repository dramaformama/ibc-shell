//! AEAD + KDF primitives. XChaCha20-Poly1305 with random 24-byte nonces (`OsRng` only);
//! Argon2id with parameters persisted next to the data they protect.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{rand_core::RngCore, OsRng, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{Result, VaultError};

/// `XChaCha20` nonce length.
pub const NONCE_LEN: usize = 24;
/// Max Argon2 memory (KiB) accepted from a backup file: 256 MiB.
pub const BACKUP_MAX_M: u32 = 262_144;
/// Max Argon2 iterations accepted from a backup file.
pub const BACKUP_MAX_T: u32 = 10;
/// Poly1305 tag length.
pub const TAG_LEN: usize = 16;

/// Fills `N` random bytes from the OS RNG.
pub fn random_bytes<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0u8; N];
    OsRng
        .try_fill_bytes(&mut out)
        .map_err(|_| VaultError::Io("os rng unavailable".into()))?;
    Ok(out)
}

/// Encrypts; returns `(nonce, ciphertext||tag)`.
pub fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<([u8; NONCE_LEN], Vec<u8>)> {
    let nonce = random_bytes::<NONCE_LEN>()?;
    let ct = seal_with_nonce(key, &nonce, aad, plaintext)?;
    Ok((nonce, ct))
}

/// Encrypts with an explicit nonce (KAT tests, backup header).
pub fn seal_with_nonce(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    use chacha20poly1305::aead::Aead;
    XChaCha20Poly1305::new(key.into())
        .encrypt(XNonce::from_slice(nonce), Payload { msg: plaintext, aad })
        .map_err(|_| VaultError::Corrupt)
}

/// Decrypts; any failure (tag, AAD, length) is [`VaultError::Corrupt`].
pub fn open(key: &[u8; 32], aad: &[u8], nonce: &[u8], ct: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    use chacha20poly1305::aead::Aead;
    if nonce.len() != NONCE_LEN || ct.len() < TAG_LEN {
        return Err(VaultError::Corrupt);
    }
    XChaCha20Poly1305::new(key.into())
        .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad })
        .map(Zeroizing::new)
        .map_err(|_| VaultError::Corrupt)
}

/// `nonce || ciphertext` blob, as stored in `vault_meta`.
pub fn seal_blob(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let (nonce, ct) = seal(key, aad, plaintext)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Opens a blob made by [`seal_blob`].
pub fn open_blob(key: &[u8; 32], aad: &[u8], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if blob.len() < NONCE_LEN {
        return Err(VaultError::Corrupt);
    }
    let (nonce, ct) = blob.split_at(NONCE_LEN);
    open(key, aad, nonce, ct)
}

/// Argon2id parameters + salt. Persisted (CBOR) so cost can be raised later.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory in KiB.
    pub m: u32,
    /// Iterations.
    pub t: u32,
    /// Lanes.
    pub p: u32,
    /// 16-byte salt.
    pub salt: Vec<u8>,
}

impl KdfParams {
    /// Contract default: m=64 MiB, t=3, p=1, fresh salt.
    pub fn default_with_salt() -> Result<Self> {
        Self::with_cost(65_536, 3, 1)
    }

    /// Custom cost with a fresh salt.
    pub fn with_cost(m: u32, t: u32, p: u32) -> Result<Self> {
        Ok(Self { m, t, p, salt: random_bytes::<16>()?.to_vec() })
    }

    /// Extra bound for params read from an (untrusted) backup file: m <= 256 MiB, t <= 10.
    pub fn check_backup_bounds(&self) -> Result<()> {
        if self.m > BACKUP_MAX_M || self.t > BACKUP_MAX_T {
            return Err(VaultError::Corrupt);
        }
        Ok(())
    }

    /// Derives a 32-byte key. Params read from disk are bounds-checked first.
    pub fn derive(&self, password: &SecretString) -> Result<Zeroizing<[u8; 32]>> {
        if self.salt.len() != 16
            || self.m < 8
            || self.m > 1_048_576
            || self.t == 0
            || self.t > 64
            || self.p == 0
            || self.p > 16
        {
            return Err(VaultError::Corrupt);
        }
        let params = Params::new(self.m, self.t, self.p, Some(32)).map_err(|_| VaultError::Corrupt)?;
        let mut out = Zeroizing::new([0u8; 32]);
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(password.expose_secret().as_bytes(), &self.salt, out.as_mut())
            .map_err(|_| VaultError::Corrupt)?;
        Ok(out)
    }

    /// CBOR encoding.
    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        ciborium::into_writer(self, &mut out).map_err(|_| VaultError::Corrupt)?;
        Ok(out)
    }

    /// CBOR decoding.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        ciborium::from_reader(bytes).map_err(|_| VaultError::Corrupt)
    }
}
