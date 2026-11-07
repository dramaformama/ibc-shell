# Contract: security
Status: `FROZEN v1` (2026-11-07). Changes only via ADR, bump to `v1.x`. Crypto parameters: ADR-003.

## Vault key hierarchy
- **DEK**: 32 random bytes (OsRng). Encrypts every secret: `XChaCha20-Poly1305(key=DEK, nonce=random 24B, aad="tl/secret/v1"||secret_id)`.
- **KEK sources** (one or both):
  - `OsKeychain`: random 32-byte KEK stored in OS keychain under service `app.ibcserver`, account `vault-kek-v1`. Android: KEK never leaves Keystore — DEK wrapped with Keystore AES-256-GCM key instead (own plugin).
  - `MasterPassword`: `Argon2id(password, salt=16B, m=65536 KiB, t=3, p=1, out=32)`. Params stored in `vault_meta` so they can be raised later. Mobile may use m=32768 if device RAM < 3 GB (record in meta).
- `vault_meta` keys: `version`=1, `mode`, `kdf` (params+salt, CBOR), `dek_wrap_pw` (nonce||ct), `dek_wrap_os` (nonce||ct), `check` (encrypt constant "tl-ok" with DEK, to verify unlock).
- Password change = rewrap DEK only. Secrets untouched.
- Unlocked DEK held in `secrecy::SecretBox<[u8;32]>`, zeroized on lock/drop. Auto-lock: idle 15 min (setting `security.autolock_minutes`, 0 = never; ADR-006), OS sleep/screen lock (desktop events where available), app backgrounded > 60 s on mobile.
- Biometric: mobile — keychain/keystore item with user-presence requirement holds KEK; desktop (Phase 3) — Touch ID via LocalAuthentication gating keychain read; Windows Hello via `UserConsentVerifier`.

## Vault public API (crates/vault/src/lib.rs, keystore.rs)
```rust
pub enum VaultState { Uninitialized, Locked, Unlocked }          // IPC VaultStatus.state
pub enum VaultMode { OsKeychain, MasterPassword }                 // IPC VaultStatus.mode / vault_init
pub struct ImportSummary { folders, hosts, identities, tags, snippets, forwards, known_hosts, skipped: u32 }
pub enum VaultError { Locked, AlreadyInitialized, Uninitialized, WrongPassword, WeakPassword, Corrupt,
                      UnsupportedVersion(u16), Store(StoreError), KeyStore(KeyStoreError), Io(String), Unimplemented }
pub struct Vault;   // Send + Sync, manual Debug; all methods blocking (Argon2) → bridge calls via spawn_blocking
impl Vault {
  pub fn new(store: Arc<Store>, keystore: Arc<dyn KeyStore>) -> Self;     // starts Locked/Uninitialized
  pub fn state(&self) -> Result<VaultState>;   pub fn mode(&self) -> Result<Option<VaultMode>>;
  pub fn biometric_available(&self) -> bool;
  pub fn init(&self, mode: VaultMode, password: Option<&SecretString>) -> Result<()>;  // leaves Unlocked
  pub fn unlock_with_password(&self, pw: &SecretString) -> Result<()>;
  pub fn unlock_with_keystore(&self) -> Result<()>;
  pub fn lock(&self);                                                      // zeroize DEK, idempotent
  pub fn change_password(&self, old: &SecretString, new: &SecretString) -> Result<()>; // rewrap only
  pub fn put_secret(&self, plaintext: &[u8]) -> Result<SecretId>;
  pub fn get_secret(&self, id: SecretId) -> Result<Zeroizing<Vec<u8>>>;
  pub fn delete_secret(&self, id: SecretId) -> Result<()>;
  pub fn export_backup(&self, path: &Path, password: &SecretString) -> Result<()>;           // ≥ 10 chars
  pub fn import_backup(&self, path: &Path, password: &SecretString) -> Result<ImportSummary>;
}
pub trait KeyStore: Send + Sync {          // keystore.rs; desktop/iOS: OsKeyring (keyring crate); Android: mobile plugin
  fn wrap_dek(&self, dek: &SecretBox<[u8; 32]>) -> Result<Vec<u8>, KeyStoreError>;   // → vault_meta.dek_wrap_os
  fn unwrap_dek(&self, wrapped: &[u8]) -> Result<SecretBox<[u8; 32]>, KeyStoreError>; // may show biometric prompt
  fn reset(&self) -> Result<(), KeyStoreError>;
  fn biometric_available(&self) -> bool;
}
pub enum KeyStoreError { NotFound, Cancelled, Platform(String), Unimplemented }
```
Bridge maps `VaultError::Locked → ApiError::VaultLocked`, `WrongPassword → WrongPassword`, `WeakPassword → Validation`, rest → `Internal` (sanitized).

## IPC secret inputs
Typed passwords / passphrases / kbd answers arrive as `SecretInput` (`#[serde(transparent)] SecretString`, TS `string`, redacted `Debug`) or `Option<SecretString>` fields with `#[specta(type = Option<String>)]`. They are never echoed, logged, or returned.

## Backup format `.tlvault`
`magic "TLVAULT\0" | u16 version=1 | kdf params (CBOR) | salt 16B | nonce 24B | ciphertext(XChaCha20-Poly1305, aad=header bytes) of zstd(CBOR{folders,hosts,identities WITH decrypted secrets,tags,snippets,forwards,known_hosts})`. Export password independent of vault password; min 10 chars, zxcvbn-style strength hint in UI.

## SSH policy
- KEX: `mlkem768x25519-sha256` (if supported by russh version), `curve25519-sha256`, `curve25519-sha256@libssh.org`, `ecdh-sha2-nistp256/384/521`, `diffie-hellman-group16-sha512`, `diffie-hellman-group14-sha256`. Strict-kex (`kex-strict-c-v00@openssh.com`) ALWAYS offered.
- Ciphers: `chacha20-poly1305@openssh.com`, `aes256-gcm@openssh.com`, `aes128-gcm@openssh.com`, `aes256-ctr`, `aes128-ctr` (ctr with etm MACs only).
- MAC: `hmac-sha2-256-etm`, `hmac-sha2-512-etm`; non-etm only with ctr if server offers nothing else.
- Host key algos: `ssh-ed25519`, `ecdsa-sha2-*`, `rsa-sha2-512`, `rsa-sha2-256`. **Disabled by default:** `ssh-rsa` (SHA-1), `diffie-hellman-group1-sha1`, `group14-sha1`, CBC, 3DES. Per-host "legacy algorithms" toggle with red warning (Phase 3).
- Host key verification:
  - Unknown → `HostKeyUnknown` → TOFU dialog showing SHA256 fingerprint + randomart; accept stores in `known_host`.
  - Changed → `HostKeyChanged` → hard block. Only path: user deletes old entry in Known hosts screen, reconnects, accepts again.
  - Jump hosts verified independently, each hop.
- Agent forwarding: OFF by default, per-host opt-in with warning. X11: not supported.
- Keepalive default 30 s, 3 missed → disconnect.

## Secrets handling rules
- Types: `secrecy::SecretString` for passwords/passphrases, `Zeroizing<Vec<u8>>` for key bytes. Decrypted private keys parsed then the PEM buffer zeroized.
- Manual `Debug` impls printing `[REDACTED]`.
- `tracing`: never log args of auth functions; release builds default level `info`, no payloads; log file rotated, 5 MiB × 3, in app log dir.
- Panics: `panic = "abort"` NOT used (we need cleanup); custom panic hook strips message in release.
- Clipboard: password copy (if ever added) auto-clears after 30 s.

## Terminal / remote-data hardening
- xterm: OSC 52 clipboard write disabled (setting, off by default); OSC 8 hyperlinks open only http/https/sftp after confirmation; window title escape sanitized (strip control chars, max 128 chars).
- Bracketed paste on; multi-line paste confirmation when line count > 1 and target is not in bracketed-paste mode.
- Remote file names: reject/replace `/`, `\0`; on local write reject `..`, absolute paths, drive letters, Windows reserved names (`CON, PRN, AUX, NUL, COM1-9, LPT1-9`), trailing dots/spaces, chars `<>:"|?*`; canonicalize destination and assert it's inside the target dir. Symlinks from remote are not followed during recursive download (copied as metadata only / skipped with notice).
- Never `{@html}` with remote data. Server banner rendered as plain text.

## WebView / Tauri
- CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'`. (`'unsafe-inline'` for style only, needed by xterm; remove if xterm version allows.)
- Capabilities: one window `main`, permissions only for used plugins and our commands. No `shell`, no `fs` plugin (FS goes through our commands with path validation), no `http`.
- Devtools disabled in release. `withGlobalTauri: false`.
- Updater: minisign public key embedded; HTTPS endpoint; reject downgrade.

## Supply chain
- `cargo deny` (licenses, bans: openssl-sys allowed only if russh needs it — prefer ring/aws-lc via ADR), `cargo audit` in CI daily, `pnpm audit --prod`, Dependabot/Renovate weekly, pinned GitHub Actions by SHA.
- SBOM: `cargo cyclonedx` + `pnpm sbom` attached to releases.

## Checklist (security-reviewer, each gate)
1. grep for `unwrap|expect|panic!` outside tests in crates/ & src-tauri/.
2. grep Debug derives on structs containing `Secret|password|passphrase|key`.
3. Any IPC DTO with secret-like fields in RESPONSE types.
4. tracing macros near auth/vault code.
5. Host key flow: changed-key path has no accept route.
6. Path sanitizer tests include traversal, Windows reserved, unicode normalization (NFC/NFD), very long names (> 255 bytes).
7. Capabilities file vs used commands (no extras).
8. CSP unchanged or ADR exists.
9. RNG: only OsRng / `getrandom`.
10. New deps reviewed (licence, maintenance, unsafe usage).
