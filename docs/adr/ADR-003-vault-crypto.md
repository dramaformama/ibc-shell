# ADR-003: Vault cryptography
Date: 2026-11-07 · Status: Accepted

## Context
Secrets (passwords, private keys, passphrases) must be encrypted at rest in SQLite and unlockable
either silently (OS keychain) or with a master password, with cheap password changes and an
encrypted portable backup. Random nonces are required (no counter state across devices/backups).

## Decision
Random 256-bit DEK (OsRng) encrypts each secret with XChaCha20-Poly1305 (`chacha20poly1305`),
24-byte random nonce, AAD `"tl/secret/v1" || secret_id`. KEK: OS keychain (via `KeyStore` trait;
Android Keystore wraps the DEK itself) and/or Argon2id (`argon2`; m=65536 KiB, t=3, p=1, 32-byte
out, 16-byte salt; mobile m=32768 when RAM < 3 GB) — params stored in `vault_meta`. Password change
rewraps the DEK only. DEK lives in `secrecy::SecretBox<[u8;32]>`, zeroized on lock/drop.
Backup `.tlvault`: header-authenticated XChaCha20-Poly1305 over zstd(CBOR) with an independent
Argon2id key (password ≥ 10 chars).

## Consequences
No AES-GCM nonce-collision risk; Argon2 cost dominates unlock (~0.3–0.5 s desktop). Malware with
user rights on an unlocked machine is out of the threat model. Contract: security.md v1 §Vault.
