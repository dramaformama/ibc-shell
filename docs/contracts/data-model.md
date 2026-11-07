# Contract: data model (crates/store)
Status: `FROZEN v1` (2026-11-07). Changes only via ADR, bump to `v1.x`.
Exact Rust types: `crates/store/src/{ids,tree,catalog,resolve,error}.rs` + `lib.rs` (skeleton) — they ARE the contract; this file explains them.

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
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]   // TS {kind:"host"} | {kind:"folder", id} | {kind:"default"}
pub enum Source { Host, Folder(FolderId), Default }   // identity-provided username reports Default

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
pub struct Store;   // Send + Sync; one writer Mutex<Connection> + read connections (no r2d2)
pub type Result<T, E = StoreError> = std::result::Result<T, E>;
pub enum StoreError { NotFound, Validation(String), Cycle, TooDeep, JumpCycle, Conflict(String), Db(String),
                      Unimplemented /* Phase 0 only; must not be returned after gate G1 */ }
impl Store {
  pub fn open(path: &Path) -> Result<Self>;            // creates + migrates; WAL; foreign_keys=ON; 0600
  pub fn open_in_memory() -> Result<Self>;
  // tree
  pub fn tree_list(&self) -> Result<Tree>;              // Tree { folders: Vec<Folder>, hosts: Vec<Host> }, sorted (parent, sort_key)
  pub fn folder_create(&self, input: FolderInput) -> Result<Folder>;      // appended last among siblings
  pub fn folder_update(&self, id: FolderId, patch: FolderPatch) -> Result<Folder>;
  pub fn folder_move(&self, id: FolderId, parent: Option<FolderId>, after: Option<NodeRef>) -> Result<()>; // after=None → first
  pub fn folder_delete(&self, id: FolderId, mode: FolderDeleteMode) -> Result<()>;
  // hosts
  pub fn host_create(&self, input: HostInput) -> Result<Host>;
  pub fn host_update(&self, id: HostId, patch: HostPatch) -> Result<Host>;   // tag_ids: Some(v) replaces the set
  pub fn host_move(&self, id: HostId, folder: Option<FolderId>, after: Option<NodeRef>) -> Result<()>;
  pub fn host_duplicate(&self, id: HostId) -> Result<Host>;                  // label + " copy", placed right after
  pub fn host_delete(&self, ids: &[HostId]) -> Result<()>;
  pub fn host_resolve(&self, id: HostId) -> Result<ResolvedHost>;
  pub fn host_search(&self, q: &str, limit: u32) -> Result<Vec<HostId>>;
  pub fn host_touch_connected(&self, id: HostId) -> Result<()>;
  // identities (metadata + secret ids; `Identity` has NO specta::Type → never sent over IPC)
  pub fn identity_list(&self) -> Result<Vec<Identity>>;
  pub fn identity_get(&self, id: IdentityId) -> Result<Identity>;
  pub fn identity_create(&self, input: IdentityMetaInput) -> Result<Identity>;
  pub fn identity_update(&self, id: IdentityId, patch: IdentityMetaPatch) -> Result<Identity>; // replaced secret rows deleted in same txn
  pub fn identity_delete(&self, id: IdentityId) -> Result<()>;                // + its vault_secret rows (invariant 5)
  // tags / snippets / forwards
  pub fn tag_list(&self) -> Result<Vec<Tag>>;  tag_create(TagInput) -> Tag;  tag_update(TagId, TagPatch) -> Tag;  tag_delete(TagId);
  pub fn snippet_list(&self, folder: Option<FolderId>) -> Result<Vec<Snippet>>;  // global + that folder
  pub fn snippet_create(SnippetInput) -> Snippet;  snippet_update(SnippetId, SnippetPatch) -> Snippet;  snippet_delete(SnippetId);
  pub fn forward_list(&self, host: Option<HostId>) -> Result<Vec<PortForward>>;
  pub fn forward_create(ForwardInput) -> PortForward;  forward_update(ForwardId, ForwardPatch) -> PortForward;  forward_delete(ForwardId);
  // known hosts
  pub fn known_host_list(&self) -> Result<Vec<KnownHostSummary>>;           // IPC DTO, no key blob
  pub fn known_host_lookup(&self, pattern: &str) -> Result<Vec<KnownHost>>; // full rows incl. key_blob
  pub fn known_host_upsert(&self, kh: KnownHostInput) -> Result<()>;        // on (host_pattern, key_type)
  pub fn known_host_delete(&self, id: KnownHostId) -> Result<()>;
  // settings (JSON)
  pub fn setting_get(&self, key: &str) -> Result<Option<serde_json::Value>>;
  pub fn setting_set(&self, key: &str, value: &serde_json::Value) -> Result<()>;
  // raw secret rows + meta for vault (vault never opens SQLite itself)
  pub fn secret_put(&self, row: SecretRow) -> Result<()>;
  pub fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>>;
  pub fn secret_delete(&self, id: SecretId) -> Result<()>;
  pub fn secret_ids(&self) -> Result<Vec<SecretId>>;
  pub fn vault_meta_get(&self, k: &str) -> Result<Option<Vec<u8>>>;
  pub fn vault_meta_set(&self, k: &str, v: &[u8]) -> Result<()>;
  // backup (.tlvault) support
  pub fn export_snapshot(&self) -> Result<Snapshot>;                         // one read txn; no secrets, no settings
  pub fn import_snapshot(&self, snapshot: Snapshot) -> Result<ImportCounts>; // merge; existing ids skipped; one txn
}
pub const KNOWN_FLAGS: &[&str];   // crates/store/src/flags_list.rs (generated by flags agent; placeholder now)
```
Note: vault depends on store's `SecretRow`/meta/snapshot API only (allowed edge `vault → store`). `ssh`/`sftp` depend on neither.
IPC field names are snake_case; `IdentityKind` serializes `password|key|key_password|agent`, `ForwardKind` `local|remote|dynamic`,
`NodeRef` `{kind:"folder"|"host", id}`, `FolderDeleteMode` `"MoveToParent"|"DeleteContents"`.

## Patch semantics
`*Patch` fields are `Option<Option<T>>`: `None` = untouched, `Some(None)` = set NULL (inherit), `Some(Some(v))` = set. Rust: `#[serde(default, deserialize_with = "store::patch::double_option")]` + `#[specta(type = Option<T>, optional)]`. TS: `field?: T | null` (absent = untouched, `null` = clear). Non-nullable patch fields (`name`, `label`, `hostname`, `expanded`) are plain `Option<T>`: absent or `null` = untouched.

## DB location
`app_data_dir()/ibc.db` (Tauri path API). File permissions 0600 on unix.

## crates/geo (optional, Phase 3)
`pub fn lookup_country(ip: IpAddr) -> Result<Option<String>, GeoError>` — lowercase alpha-2, `None` for private/unknown; `GeoError::{Unavailable, Unimplemented}`. Feature-gated, off by default; not exposed over IPC in v1.
