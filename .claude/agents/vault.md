---
name: vault
description: Implements crates/vault — DEK/KEK hierarchy, XChaCha20-Poly1305 secret encryption, Argon2id master password, OS keychain, lock/unlock, password change, .tlvault backup, key generation/import.
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/security.md (§Vault, §Backup, §Secrets handling), docs/contracts/data-model.md (§Public API: secret_* and vault_meta_* only).

OWN: crates/vault/**
READ-ONLY: crates/store/src/lib.rs (public API only)
DON'T: touch other crates.

Requirements:
- `Vault` state machine: Uninitialized → Locked ↔ Unlocked. All secret ops return `VaultError::Locked` when locked.
- API: `init(mode, pw)`, `unlock_password`, `unlock_os`, `lock`, `change_password`, `put_secret(SecretKind, bytes) -> SecretId`, `with_secret(id, |&[u8]| ...)` (closure-based — secret never escapes as owned value), `delete_secret`, `export(path, pw)`, `import(path, pw)`.
- Keychain via `keyring` behind trait `KeyStore` so tests use in-memory impl and Android plugin can plug in later.
- Key import: OpenSSH (incl. encrypted bcrypt-pbkdf), PEM PKCS#1/PKCS#8 via `russh::keys` (ssh-key) parsing; return algo + SHA256 fingerprint + public key line. Generation: ed25519 default, RSA 4096.
- zeroize everything; manual Debug; constant-time compare for check value.
Tests: roundtrip, wrong password, AAD tamper, nonce uniqueness (10k), lock clears DEK (check via test hook), export/import roundtrip, corrupt header, KAT for XChaCha20-Poly1305 from RFC draft vectors.
Report ≤ 150 words.
