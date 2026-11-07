//! Migration 001 (data-model.md). PRAGMAs are applied per connection in `db.rs`.

use rusqlite_migration::{Migrations, M};

const V1: &str = r"
CREATE TABLE folder (
  id BLOB PRIMARY KEY,
  parent_id BLOB REFERENCES folder(id) ON DELETE RESTRICT,
  name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 128),
  country_code TEXT CHECK(country_code IS NULL OR length(country_code) BETWEEN 2 AND 6),
  color TEXT,
  icon TEXT,
  sort_key TEXT NOT NULL,
  expanded INTEGER NOT NULL DEFAULT 1,
  d_username TEXT, d_port INTEGER, d_identity_id BLOB REFERENCES identity(id) ON DELETE SET NULL,
  d_jump_host_id BLOB REFERENCES host(id) ON DELETE SET NULL,
  d_startup_cmd TEXT, d_keepalive_s INTEGER, d_term_theme TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE host (
  id BLOB PRIMARY KEY,
  folder_id BLOB REFERENCES folder(id) ON DELETE RESTRICT,
  label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 128),
  hostname TEXT NOT NULL,
  port INTEGER,
  username TEXT,
  identity_id BLOB REFERENCES identity(id) ON DELETE SET NULL,
  jump_host_id BLOB REFERENCES host(id) ON DELETE SET NULL,
  country_code TEXT,
  startup_cmd TEXT, keepalive_s INTEGER, term_theme TEXT,
  notes TEXT,
  sort_key TEXT NOT NULL,
  last_connected_at INTEGER,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE tag (id BLOB PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT);
CREATE TABLE host_tag (host_id BLOB REFERENCES host(id) ON DELETE CASCADE,
                       tag_id BLOB REFERENCES tag(id) ON DELETE CASCADE, PRIMARY KEY(host_id, tag_id));

CREATE TABLE identity (
  id BLOB PRIMARY KEY,
  label TEXT NOT NULL,
  username TEXT,
  kind TEXT NOT NULL CHECK(kind IN ('password','key','key_password','agent')),
  key_algo TEXT,
  key_fingerprint TEXT,
  public_key TEXT,
  password_secret_id BLOB, key_secret_id BLOB, passphrase_secret_id BLOB,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE vault_secret (
  id BLOB PRIMARY KEY,
  nonce BLOB NOT NULL,
  ciphertext BLOB NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE vault_meta (k TEXT PRIMARY KEY, v BLOB NOT NULL);

CREATE TABLE known_host (
  id BLOB PRIMARY KEY,
  host_pattern TEXT NOT NULL,
  key_type TEXT NOT NULL,
  key_blob BLOB NOT NULL,
  fingerprint TEXT NOT NULL,
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

CREATE TABLE setting (k TEXT PRIMARY KEY, v TEXT NOT NULL);

CREATE VIRTUAL TABLE host_fts USING fts5(label, hostname, tags, notes, content='', tokenize='unicode61 remove_diacritics 2');
CREATE INDEX idx_host_folder ON host(folder_id, sort_key);
CREATE INDEX idx_folder_parent ON folder(parent_id, sort_key);
";

/// All migrations, in order.
pub(crate) fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1)])
}
