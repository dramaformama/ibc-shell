//! Vault unit tests against an in-memory storage (the real `Store` lands in P1.STORE).
#![allow(
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::format_collect
)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use ibc_shell_store::{ImportCounts, SecretId, SecretRow, Snapshot, StoreError};
use secrecy::SecretString;

use crate::crypto;
use crate::keys::{generate_key, import_private_key, KeyError, KeyGenAlgo};
use crate::*;

#[derive(Default)]
struct MemStorage {
    meta: Mutex<HashMap<String, Vec<u8>>>,
    rows: Mutex<HashMap<SecretId, SecretRow>>,
    snap: Mutex<Snapshot>,
}

impl VaultStorage for MemStorage {
    fn meta_get(&self, k: &str) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.meta.lock().unwrap().get(k).cloned())
    }
    fn meta_set(&self, k: &str, v: &[u8]) -> Result<(), StoreError> {
        self.meta.lock().unwrap().insert(k.into(), v.to_vec());
        Ok(())
    }
    fn secret_put(&self, row: SecretRow) -> Result<(), StoreError> {
        self.rows.lock().unwrap().insert(row.id, row);
        Ok(())
    }
    fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>, StoreError> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }
    fn secret_delete(&self, id: SecretId) -> Result<(), StoreError> {
        self.rows.lock().unwrap().remove(&id);
        Ok(())
    }
    fn secret_ids(&self) -> Result<Vec<SecretId>, StoreError> {
        Ok(self.rows.lock().unwrap().keys().copied().collect())
    }
    fn export_snapshot(&self) -> Result<Snapshot, StoreError> {
        Ok(self.snap.lock().unwrap().clone())
    }
    fn import_snapshot(&self, _s: Snapshot) -> Result<ImportCounts, StoreError> {
        Ok(ImportCounts::default())
    }
}

fn pw(s: &str) -> SecretString {
    SecretString::from(s.to_owned())
}

fn fast(st: &Arc<MemStorage>, ks: &Arc<MemoryKeyStore>) -> Vault {
    Vault::with_storage(st.clone(), ks.clone()).with_kdf_cost(64, 1, 1)
}

fn setup() -> (Arc<MemStorage>, Arc<MemoryKeyStore>, Vault) {
    let st = Arc::new(MemStorage::default());
    let ks = Arc::new(MemoryKeyStore::new());
    let v = fast(&st, &ks);
    (st, ks, v)
}

#[test]
fn xchacha_kat_draft_irtf_cfrg_xchacha_a31() {
    let key: [u8; 32] = std::array::from_fn(|i| 0x80 + i as u8);
    let nonce: [u8; 24] = std::array::from_fn(|i| 0x40 + i as u8);
    let aad = [
        0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    ];
    let pt = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let expect = "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b4522f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff921f9664c97637da9768812f615c68b13b52ec0875924c1c7987947deafd8780acf49";
    let ct = crypto::seal_with_nonce(&key, &nonce, &aad, pt).unwrap();
    let hex: String = ct.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, expect);
    assert_eq!(
        crypto::open(&key, &aad, &nonce, &ct).unwrap().as_slice(),
        pt
    );
}

#[test]
fn password_roundtrip_and_states() {
    let (_st, _ks, v) = setup();
    assert_eq!(v.state().unwrap(), VaultState::Uninitialized);
    assert!(matches!(v.put_secret(b"x"), Err(VaultError::Locked)));
    v.init(VaultMode::MasterPassword, Some(&pw("correct horse")))
        .unwrap();
    assert_eq!(v.state().unwrap(), VaultState::Unlocked);
    assert_eq!(v.mode().unwrap(), Some(VaultMode::MasterPassword));
    let id = v.put_secret(b"hunter2").unwrap();
    assert_eq!(v.with_secret(id, <[u8]>::to_vec).unwrap(), b"hunter2");
    v.lock();
    assert_eq!(v.state().unwrap(), VaultState::Locked);
    assert!(matches!(v.get_secret(id), Err(VaultError::Locked)));
    v.unlock_with_password(&pw("correct horse")).unwrap();
    assert_eq!(v.get_secret(id).unwrap().as_slice(), b"hunter2");
    assert!(matches!(
        v.init(VaultMode::OsKeychain, None),
        Err(VaultError::AlreadyInitialized)
    ));
}

#[test]
fn wrong_password_and_weak_password() {
    let (_st, _ks, v) = setup();
    assert!(matches!(
        v.init(VaultMode::MasterPassword, Some(&pw("short"))),
        Err(VaultError::WeakPassword)
    ));
    assert!(matches!(
        v.init(VaultMode::MasterPassword, None),
        Err(VaultError::WeakPassword)
    ));
    assert_eq!(v.state().unwrap(), VaultState::Uninitialized);
    v.init(VaultMode::MasterPassword, Some(&pw("correct horse")))
        .unwrap();
    v.lock();
    assert!(matches!(
        v.unlock_with_password(&pw("wrong wrong")),
        Err(VaultError::WrongPassword)
    ));
    assert_eq!(v.state().unwrap(), VaultState::Locked);
}

#[test]
fn os_keychain_mode_roundtrip() {
    let (st, ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let id = v.put_secret(b"s").unwrap();
    v.lock();
    v.unlock_with_keystore().unwrap();
    assert_eq!(v.get_secret(id).unwrap().as_slice(), b"s");
    // Keystore reset (key deleted) makes unlock fail.
    v.lock();
    ks.reset().unwrap();
    assert!(matches!(
        v.unlock_with_keystore(),
        Err(VaultError::KeyStore(KeyStoreError::NotFound))
    ));
    let _ = st;
}

#[test]
fn change_password_rewraps_only() {
    let (st, _ks, v) = setup();
    v.init(VaultMode::MasterPassword, Some(&pw("old password")))
        .unwrap();
    let id = v.put_secret(b"payload").unwrap();
    let before = st.secret_get(id).unwrap().unwrap();
    assert!(matches!(
        v.change_password(&pw("nope nope"), &pw("new password")),
        Err(VaultError::WrongPassword)
    ));
    v.change_password(&pw("old password"), &pw("new password"))
        .unwrap();
    let after = st.secret_get(id).unwrap().unwrap();
    assert_eq!(before, after, "secret rows untouched");
    v.lock();
    assert!(matches!(
        v.unlock_with_password(&pw("old password")),
        Err(VaultError::WrongPassword)
    ));
    v.unlock_with_password(&pw("new password")).unwrap();
    assert_eq!(v.get_secret(id).unwrap().as_slice(), b"payload");
}

#[test]
fn aad_binds_ciphertext_to_secret_id() {
    let (st, _ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let a = v.put_secret(b"alpha").unwrap();
    let b = v.put_secret(b"beta").unwrap();
    let ra = st.secret_get(a).unwrap().unwrap();
    // Move A's ciphertext under B's id.
    st.secret_put(SecretRow { id: b, ..ra }).unwrap();
    assert!(matches!(v.get_secret(b), Err(VaultError::Corrupt)));
    // Bit flip.
    let mut rb = st.secret_get(a).unwrap().unwrap();
    rb.ciphertext[0] ^= 1;
    st.secret_put(rb).unwrap();
    assert!(matches!(v.get_secret(a), Err(VaultError::Corrupt)));
}

#[test]
fn nonces_unique_over_10k() {
    let (st, _ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let mut seen = HashSet::new();
    for _ in 0..10_000 {
        let id = v.put_secret(b"x").unwrap();
        let row = st.secret_get(id).unwrap().unwrap();
        assert_eq!(row.nonce.len(), 24);
        assert!(seen.insert(row.nonce));
    }
}

#[test]
fn lock_clears_dek() {
    let (_st, _ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    assert!(v.dek_loaded());
    v.lock();
    assert!(!v.dek_loaded());
    v.lock();
    assert!(!v.dek_loaded());
}

#[test]
fn corrupt_meta_is_detected() {
    let (st, ks, v) = setup();
    v.init(VaultMode::MasterPassword, Some(&pw("correct horse")))
        .unwrap();
    v.lock();
    // Unsupported version.
    st.meta_set("version", &[0, 9]).unwrap();
    assert!(matches!(
        v.unlock_with_password(&pw("correct horse")),
        Err(VaultError::UnsupportedVersion(9))
    ));
    st.meta_set("version", &[0, 1]).unwrap();
    // Tampered check value.
    let good = st.meta_get("check").unwrap().unwrap();
    let mut bad = good.clone();
    bad[30] ^= 1;
    st.meta_set("check", &bad).unwrap();
    assert!(matches!(
        v.unlock_with_password(&pw("correct horse")),
        Err(VaultError::Corrupt)
    ));
    st.meta_set("check", &good).unwrap();
    // Garbage KDF params.
    st.meta_set("kdf", b"\xff\xff").unwrap();
    assert!(matches!(
        v.unlock_with_password(&pw("correct horse")),
        Err(VaultError::Corrupt)
    ));
    // Missing wrap.
    let v2 = fast(&st, &ks);
    st.meta.lock().unwrap().remove("dek_wrap_pw");
    assert!(matches!(
        v2.unlock_with_password(&pw("correct horse")),
        Err(VaultError::Corrupt)
    ));
}

#[test]
fn debug_does_not_leak() {
    let (_st, ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let s = format!("{v:?} {ks:?}");
    assert!(s.contains("REDACTED"));
    let g = generate_key(KeyGenAlgo::Ed25519).unwrap();
    assert!(!format!("{g:?}").contains("BEGIN"));
}

#[test]
fn backup_roundtrip_and_failures() {
    let (st, _ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let id = v.put_secret(b"top secret").unwrap();
    let dir = std::env::temp_dir().join(format!("tlvault-test-{}", SecretId::new_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("b.tlvault");
    assert!(matches!(
        v.export_backup(&path, &pw("short")),
        Err(VaultError::WeakPassword)
    ));
    v.export_backup(&path, &pw("backup password")).unwrap();

    // Import into a fresh vault: secret re-encrypted under the new DEK, same id.
    let (st2, _ks2, v2) = setup();
    v2.init(VaultMode::OsKeychain, None).unwrap();
    v2.import_backup(&path, &pw("backup password")).unwrap();
    assert_eq!(v2.get_secret(id).unwrap().as_slice(), b"top secret");
    assert_ne!(
        st.secret_get(id).unwrap().unwrap().ciphertext,
        st2.secret_get(id).unwrap().unwrap().ciphertext
    );

    assert!(matches!(
        v2.import_backup(&path, &pw("wrong password!")),
        Err(VaultError::WrongPassword)
    ));
    v2.lock();
    assert!(matches!(
        v2.import_backup(&path, &pw("backup password")),
        Err(VaultError::Locked)
    ));
    v2.unlock_with_keystore().unwrap();

    // Header tamper, ciphertext tamper, bad magic, truncation.
    let good = std::fs::read(&path).unwrap();
    for (name, f) in [
        (
            "hdr",
            Box::new(|b: &mut Vec<u8>| b[20] ^= 1) as Box<dyn Fn(&mut Vec<u8>)>,
        ),
        (
            "ct",
            Box::new(|b: &mut Vec<u8>| {
                let n = b.len() - 3;
                b[n] ^= 1;
            }),
        ),
        ("magic", Box::new(|b: &mut Vec<u8>| b[0] = b'X')),
        ("trunc", Box::new(|b: &mut Vec<u8>| b.truncate(10))),
    ] {
        let mut bad = good.clone();
        f(&mut bad);
        let p = dir.join(format!("{name}.tlvault"));
        std::fs::write(&p, &bad).unwrap();
        assert!(
            matches!(
                v2.import_backup(&p, &pw("backup password")),
                Err(VaultError::Corrupt | VaultError::WrongPassword)
            ),
            "{name}"
        );
    }
    let mut ver = good.clone();
    ver[9] = 7;
    let p = dir.join("ver.tlvault");
    std::fs::write(&p, &ver).unwrap();
    assert!(matches!(
        v2.import_backup(&p, &pw("backup password")),
        Err(VaultError::UnsupportedVersion(7))
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn key_generate_and_import() {
    let g = generate_key(KeyGenAlgo::Ed25519).unwrap();
    assert_eq!(g.info.algo, "ssh-ed25519");
    assert!(g.info.fingerprint.starts_with("SHA256:"));
    let info = import_private_key(&g.private_openssh, None).unwrap();
    assert_eq!(info, g.info);
    assert!(matches!(
        import_private_key(b"garbage", None),
        Err(KeyError::Invalid)
    ));
}

#[test]
fn backup_rejects_oversized_kdf_params_and_survives_stale_tmp() {
    use crate::crypto::KdfParams;
    let (_st, _ks, v) = setup();
    v.init(VaultMode::OsKeychain, None).unwrap();
    let dir = std::env::temp_dir().join(format!("tlvault-test-{}", SecretId::new_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("b.tlvault");

    // Stale tmp (even a symlink to a victim file) is replaced, never written through.
    let victim = dir.join("victim");
    std::fs::write(&victim, b"keep").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&victim, path.with_extension("tlvault.tmp")).unwrap();
    #[cfg(not(unix))]
    std::fs::write(path.with_extension("tlvault.tmp"), b"stale").unwrap();
    v.export_backup(&path, &pw("backup password")).unwrap();
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    // Header claiming m = 512 MiB is refused before any key derivation.
    let good = std::fs::read(&path).unwrap();
    let kdf_len = u32::from_be_bytes([good[10], good[11], good[12], good[13]]) as usize;
    let mut p = KdfParams::from_cbor(&good[14..14 + kdf_len]).unwrap();
    p.m = 524_288;
    let cbor = p.to_cbor().unwrap();
    let mut bad = good[..10].to_vec();
    bad.extend_from_slice(&u32::try_from(cbor.len()).unwrap().to_be_bytes());
    bad.extend_from_slice(&cbor);
    bad.extend_from_slice(&good[14 + kdf_len..]);
    let bp = dir.join("big.tlvault");
    std::fs::write(&bp, &bad).unwrap();
    assert!(matches!(
        v.import_backup(&bp, &pw("backup password")),
        Err(VaultError::Corrupt)
    ));
    p.m = 65_536;
    p.t = 11;
    assert!(p.check_backup_bounds().is_err());
    p.t = 10;
    p.m = 262_144;
    assert!(p.check_backup_bounds().is_ok());
    std::fs::remove_dir_all(dir).unwrap();
}
