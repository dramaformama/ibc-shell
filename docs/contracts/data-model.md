# Contract: data model (crates/store)
Status: DRAFT → architect sets `FROZEN v1` at end of Phase 0.

## IDs & time
- All IDs: UUID v7, stored as 16-byte BLOB, exposed to UI as lowercase hyphenated string.
- Timestamps: i64 unix millis (UTC).
- Sort: `sort_key TEXT` — fractional index (base-62 string, e.g. `a0`, `a0V`). Insert between neighbours without renumbering.

## Tables (migration 001)
```sql
PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;

CREATE TABLE folder (
  id BLOB PRIMARY KEY,
  parent_id BLOB REFERENCES folder(id) ON DELETE RESTRICT,   -- NULL = root
  name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 128),
  country_code TEXT CHECK(country_code IS NULL OR length(country_code) BETWEEN 2 AND 6), -- ISO 3166-1 alpha-2 lowercase, or circle-flags extras: 'eu','un','xx'
  color TEXT,                       -- optional accent, one of ui palette tokens
  icon TEXT,                        -- optional lucide icon name
  sort_key TEXT NOT NULL,
  expanded INTEGER NOT NULL DEFAULT 1,
  -- inheritable defaults (NULL = inherit from parent)
  d_username TEXT, d_port INTEGER, d_identity_id BLOB REFERENCES identity(id) ON DELETE SET NULL,
  d_jump_host_id BLOB REFERENCES host(id) ON DELETE SET NULL,
  d_startup_cmd TEXT, d_keepalive_s INTEGER, d_term_theme TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE host (
  id BLOB PRIMARY KEY,
  folder_id BLOB REFERENCES folder(id) ON DELETE RESTRICT,   -- NULL = root
  label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 128),
  hostname TEXT NOT NULL,           -- DNS name, IPv4 or IPv6 (no brackets)
  port INTEGER,                     -- NULL = inherit, final default 22
  username TEXT,                    -- NULL = inherit
  identity_id BLOB REFERENCES identity(id) ON DELETE SET NULL,
  jump_host_id BLOB REFERENCES host(id) ON DELETE SET NULL,
  country_code TEXT,                -- NULL = inherit folder flag
  startup_cmd TEXT, keepalive_s INTEGER, term_theme TEXT,
  notes TEXT,
  sort_key TEXT NOT NULL,
  last_connected_at INTEGER,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE tag (id BLOB PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT);
CREATE TABLE host_tag (host_id BLOB REFERENCES host(id) ON DELETE CASCADE,
                       tag_id BLOB REFERENCES tag(id) ON DELETE CASCADE, PRIMARY KEY(host_id, tag_id));

-- Credentials metadata only. Secret material lives in vault_secret.
CREATE TABLE identity (
  id BLOB PRIMARY KEY,
  label TEXT NOT NULL,
  username TEXT,                    -- optional; host/folder username wins if set
  kind TEXT NOT NULL CHECK(kind IN ('password','key','key_password','agent')),
  key_algo TEXT,                    -- 'ssh-ed25519' | 'rsa-sha2-512' | 'ecdsa-sha2-nistp256' ...
  key_fingerprint TEXT,             -- SHA256:base64, public info
  public_key TEXT,                  -- OpenSSH one-line public key, public info
  password_secret_id BLOB, key_secret_id BLOB, passphrase_secret_id BLOB,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE vault_secret (
  id BLOB PRIMARY KEY,
  nonce BLOB NOT NULL,              -- 24 bytes
  ciphertext BLOB NOT NULL,         -- XChaCha20-Poly1305, AAD = "tl/secret/v1" || id
  created_at INTEGER NOT NULL
);

CREATE TABLE vault_meta (k TEXT PRIMARY KEY, v BLOB NOT NULL);  -- see security.md

CREATE TABLE known_host (
  id BLOB PRIMARY KEY,
  host_pattern TEXT NOT NULL,       -- "[host]:port" OpenSSH form; port 22 → bare host
  key_type TEXT NOT NULL,
  key_blob BLOB NOT NULL,           -- SSH wire-format public key
  fingerprint TEXT NOT NULL,        -- SHA256:...
  added_at INTEGER NOT NULL,
  UNIQUE(host_pattern, key_type)
);

CREATE TABLE snippet (id BLOB PRIMARY KEY, folder_id BLOB REFERENCES folder(id) ON DELETE CASCADE,
  name TEXT NOT NULL, body TEXT NOT NULL, sort_key TEXT NOT NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);

CREATE TABLE port_forward (id BLOB PRIMARY KEY, host_id BLOB NOT NULL REFERENCES host(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('local','remote','dynamic')),
  bind_addr TEXT NOT NULL DEFAULT '127.0.0.1', bind_port INTEGER NOT NULL,
  dest_host TEXT, dest_port INTEGER, auto_start INTEGER NOT NULL DEFAULT 0, label TEXT);

CREATE TABLE setting (k TEXT PRIMARY KEY, v TEXT NOT NULL);   -- JSON values

CREATE VIRTUAL TABLE host_fts USING fts5(label, hostname, tags, notes, content='', tokenize='unicode61 remove_diacritics 2');
CREATE INDEX idx_host_folder ON host(folder_id, sort_key);
CREATE INDEX idx_folder_parent ON folder(parent_id, sort_key);
```

## Invariants
1. Folder tree is acyclic: `folder_move(id, new_parent)` rejects if `new_parent` is `id` or a descendant → `StoreError::Cycle`.
2. Max folder depth 8 → `StoreError::TooDeep`.
3. Jump chain: `jump_host_id` resolution follows inherited values, max 3 hops, no cycles → `StoreError::JumpCycle`.
4. Deleting a folder: `FolderDeleteMode::{MoveToParent, DeleteContents}`; `DeleteContents` also deletes vault secrets of identities ONLY if no other host references them (identities are global, so normally untouched).
5. Deleting an identity deletes its `vault_secret` rows in the same transaction.
6. `country_code` normalized to lowercase; validated against the flag registry list exported by store as `const KNOWN_FLAGS: &[&str]` (generated from circle-flags list by `flags` agent → `crates/store/src/flags_list.rs`).

## Inheritance resolution
```rust
pub struct Resolved<T> { pub value: T, pub source: Source }
pub enum Source { Host, Folder(FolderId), Default }

pub struct ResolvedHost {
  pub id: HostId, pub label: String, pub hostname: String,
  pub port: Resolved<u16>,               // default 22
  pub username: Resolved<Option<String>>,// None → prompt in UI
  pub identity_id: Resolved<Option<IdentityId>>,
  pub jump_chain: Vec<HostId>,           // outermost first, already resolved
  pub country_code: Resolved<Option<String>>,
  pub startup_cmd: Resolved<Option<String>>,
  pub keepalive_s: Resolved<u32>,        // default 30
  pub term_theme: Resolved<Option<String>>,
}
```
Order: host field → nearest folder up the chain with non-NULL `d_*` (for flag: `country_code`) → default. Username special case: host.username → folder d_username → identity.username → None.

## Public API (crates/store/src/lib.rs)
```rust
pub struct Store;                       // owns a connection pool (r2d2-free: one writer Mutex<Connection> + read connections)
impl Store {
  pub fn open(path: &Path) -> Result<Self, StoreError>;
  pub fn open_in_memory() -> Result<Self, StoreError>;
  // tree
  pub fn tree_list(&self) -> Result<Tree, StoreError>;          // flat Vec<Node> with parent ids, sorted
  pub fn folder_create(&self, input: FolderInput) -> Result<Folder, StoreError>;
  pub fn folder_update(&self, id: FolderId, patch: FolderPatch) -> Result<Folder, StoreError>;
  pub fn folder_move(&self, id: FolderId, parent: Option<FolderId>, after: Option<NodeRef>) -> Result<(), StoreError>;
  pub fn folder_delete(&self, id: FolderId, mode: FolderDeleteMode) -> Result<(), StoreError>;
  // hosts
  pub fn host_create(&self, input: HostInput) -> Result<Host, StoreError>;
  pub fn host_update(&self, id: HostId, patch: HostPatch) -> Result<Host, StoreError>;
  pub fn host_move(&self, id: HostId, folder: Option<FolderId>, after: Option<NodeRef>) -> Result<(), StoreError>;
  pub fn host_duplicate(&self, id: HostId) -> Result<Host, StoreError>;
  pub fn host_delete(&self, ids: &[HostId]) -> Result<(), StoreError>;
  pub fn host_resolve(&self, id: HostId) -> Result<ResolvedHost, StoreError>;
  pub fn host_search(&self, q: &str, limit: u32) -> Result<Vec<HostId>, StoreError>;
  pub fn host_touch_connected(&self, id: HostId) -> Result<(), StoreError>;
  // identities (metadata), tags, snippets, forwards, settings, known_hosts: CRUD in same style
  pub fn known_host_lookup(&self, pattern: &str) -> Result<Vec<KnownHost>, StoreError>;
  pub fn known_host_upsert(&self, kh: KnownHostInput) -> Result<(), StoreError>;
  // raw secret rows for vault (vault never opens SQLite itself)
  pub fn secret_put(&self, row: SecretRow) -> Result<(), StoreError>;
  pub fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>, StoreError>;
  pub fn secret_delete(&self, id: SecretId) -> Result<(), StoreError>;
  pub fn vault_meta_get(&self, k: &str) -> Result<Option<Vec<u8>>, StoreError>;
  pub fn vault_meta_set(&self, k: &str, v: &[u8]) -> Result<(), StoreError>;
}
```
Note: vault depends on store's `SecretRow`/meta API only (allowed edge `vault → store`). `ssh`/`sftp` depend on neither.

## Patch semantics
`*Patch` fields are `Option<Option<T>>`: `None` = untouched, `Some(None)` = set NULL (inherit), `Some(Some(v))` = set. Serialized to TS as `T | null | undefined` (undefined = untouched).

## DB location
`app_data_dir()/termlite.db` (Tauri path API). File permissions 0600 on unix.
