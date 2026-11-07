//! `.tlvault` backup: `magic "TLVAULT\0" | u16 version | u32 kdf_len | kdf CBOR (incl. salt) |
//! nonce 24B | XChaCha20-Poly1305(aad = all preceding header bytes, zstd(CBOR payload))`.

use std::io::{Read, Write};
use std::path::Path;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use termlite_store::{SecretId, Snapshot};
use zeroize::{Zeroize, Zeroizing};

use crate::crypto::{self, KdfParams, NONCE_LEN};
use crate::{ImportSummary, Result, Vault, VaultError};

const MAGIC: &[u8; 8] = b"TLVAULT\0";
const VERSION: u16 = 1;
const MIN_BACKUP_PASSWORD_CHARS: usize = 10;
const MAX_KDF_LEN: usize = 1024;
/// Decompression bomb guard.
const MAX_PLAIN: u64 = 512 * 1024 * 1024;

/// One decrypted secret inside the payload; zeroized on drop.
#[derive(Serialize, Deserialize)]
struct BackupSecret {
    id: SecretId,
    data: Vec<u8>,
}

impl Drop for BackupSecret {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

#[derive(Serialize, Deserialize)]
struct Payload {
    snapshot: Snapshot,
    secrets: Vec<BackupSecret>,
}

fn io_err(_: std::io::Error) -> VaultError {
    VaultError::Io("file operation failed".into())
}

pub(crate) fn export(vault: &Vault, path: &Path, password: &SecretString) -> Result<()> {
    vault.with_dek(|_| Ok(()))?;
    if password.expose_secret().chars().count() < MIN_BACKUP_PASSWORD_CHARS {
        return Err(VaultError::WeakPassword);
    }
    let snapshot = vault.storage().export_snapshot()?;
    let mut secrets = Vec::new();
    for id in vault.storage().secret_ids()? {
        let data = vault.get_secret(id)?;
        secrets.push(BackupSecret { id, data: data.to_vec() });
    }
    let payload = Payload { snapshot, secrets };
    let mut cbor = Zeroizing::new(Vec::new());
    ciborium::into_writer(&payload, &mut *cbor).map_err(|_| VaultError::Corrupt)?;
    let compressed = Zeroizing::new(zstd::encode_all(cbor.as_slice(), 3).map_err(io_err)?);

    let params = KdfParams::default_with_salt()?;
    let kdf = params.to_cbor()?;
    let nonce = crypto::random_bytes::<NONCE_LEN>()?;
    let mut header = Vec::with_capacity(14 + kdf.len() + NONCE_LEN);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&VERSION.to_be_bytes());
    header.extend_from_slice(&u32::try_from(kdf.len()).map_err(|_| VaultError::Corrupt)?.to_be_bytes());
    header.extend_from_slice(&kdf);
    header.extend_from_slice(&nonce);

    let key = params.derive(password)?;
    let ct = crypto::seal_with_nonce(&key, &nonce, &header, &compressed)?;

    let tmp = path.with_extension("tlvault.tmp");
    let write = || -> std::io::Result<()> {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
        let mut f = opts.open(&tmp)?;
        f.write_all(&header)?;
        f.write_all(&ct)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io_err(e)
    })
}

/// Parses + decrypts a backup file into its payload.
fn decrypt(path: &Path, password: &SecretString) -> Result<Payload> {
    let bytes = std::fs::read(path).map_err(io_err)?;
    if bytes.len() < 14 || &bytes[..8] != MAGIC {
        return Err(VaultError::Corrupt);
    }
    let version = u16::from_be_bytes([bytes[8], bytes[9]]);
    if version != VERSION {
        return Err(VaultError::UnsupportedVersion(version));
    }
    let kdf_len = u32::from_be_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
    let header_len = 14usize
        .checked_add(kdf_len)
        .and_then(|n| n.checked_add(NONCE_LEN))
        .ok_or(VaultError::Corrupt)?;
    if kdf_len > MAX_KDF_LEN || bytes.len() < header_len + crypto::TAG_LEN {
        return Err(VaultError::Corrupt);
    }
    let params = KdfParams::from_cbor(&bytes[14..14 + kdf_len])?;
    let key = params.derive(password)?;
    let (header, ct) = bytes.split_at(header_len);
    let nonce = &header[header_len - NONCE_LEN..];
    // Header is AAD: tampering anywhere (or a wrong password) fails the tag.
    let compressed = crypto::open(&key, header, nonce, ct).map_err(|_| VaultError::WrongPassword)?;

    let mut plain = Zeroizing::new(Vec::new());
    let decoder = zstd::stream::read::Decoder::new(compressed.as_slice()).map_err(io_err)?;
    decoder.take(MAX_PLAIN + 1).read_to_end(&mut plain).map_err(|_| VaultError::Corrupt)?;
    if plain.len() as u64 > MAX_PLAIN {
        return Err(VaultError::Corrupt);
    }
    ciborium::from_reader(plain.as_slice()).map_err(|_| VaultError::Corrupt)
}

pub(crate) fn import(vault: &Vault, path: &Path, password: &SecretString) -> Result<ImportSummary> {
    vault.with_dek(|_| Ok(()))?;
    let mut payload = decrypt(path, password)?;
    let snap = std::mem::take(&mut payload.snapshot);
    let total = snap.folders.len()
        + snap.hosts.len()
        + snap.identities.len()
        + snap.tags.len()
        + snap.snippets.len()
        + snap.forwards.len()
        + snap.known_hosts.len();

    // Re-encrypt secrets under this vault's DEK, keeping ids (identities refer to them).
    let mut added = Vec::new();
    for s in &payload.secrets {
        if vault.storage().secret_get(s.id)?.is_none() {
            vault.put_secret_with_id(s.id, &s.data)?;
            added.push(s.id);
        }
    }
    let counts = match vault.storage().import_snapshot(snap) {
        Ok(c) => c,
        Err(e) => {
            for id in added {
                let _ = vault.storage().secret_delete(id);
            }
            return Err(e.into());
        }
    };
    let written = [
        counts.folders,
        counts.hosts,
        counts.identities,
        counts.tags,
        counts.snippets,
        counts.forwards,
        counts.known_hosts,
    ]
    .iter()
    .map(|&n| n as usize)
    .sum::<usize>();
    Ok(ImportSummary {
        folders: counts.folders,
        hosts: counts.hosts,
        identities: counts.identities,
        tags: counts.tags,
        snippets: counts.snippets,
        forwards: counts.forwards,
        known_hosts: counts.known_hosts,
        skipped: u32::try_from(total.saturating_sub(written)).unwrap_or(u32::MAX),
    })
}
