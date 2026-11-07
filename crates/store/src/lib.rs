//! SQLite storage: models, migrations, folder tree with inheritance.
//!
//! Contract: `docs/contracts/data-model.md` (FROZEN v1). Phase 0 skeleton — every method
//! returns [`StoreError::Unimplemented`].
// Patch semantics (data-model.md) use `Option<Option<T>>` by contract.
#![allow(clippy::option_option)]
// Skeleton-only allowances; the store agent removes this line when implementing.
#![allow(unused_variables, clippy::unused_self, clippy::needless_pass_by_value)]

mod catalog;
mod error;
pub mod flags_list;
mod ids;
pub mod patch;
mod resolve;
mod tree;

use std::path::Path;

pub use catalog::{
    ForwardInput, ForwardKind, ForwardPatch, Identity, IdentityKind, IdentityMetaInput,
    IdentityMetaPatch, KnownHost, KnownHostInput, KnownHostSummary, PortForward, SecretRow,
    Snapshot, Snippet, SnippetInput, SnippetPatch, Tag, TagInput, TagPatch,
};
pub use error::StoreError;
pub use flags_list::KNOWN_FLAGS;
pub use ids::{FolderId, ForwardId, HostId, IdentityId, KnownHostId, SecretId, SnippetId, TagId};
pub use resolve::{Resolved, ResolvedHost, Source};
pub use tree::{
    Folder, FolderDefaults, FolderDefaultsPatch, FolderDeleteMode, FolderInput, FolderPatch, Host,
    HostInput, HostPatch, NodeRef, Tree,
};

/// Result alias.
pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// Counts of rows written by [`Store::import_snapshot`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImportCounts {
    /// Folders.
    pub folders: u32,
    /// Hosts.
    pub hosts: u32,
    /// Identities.
    pub identities: u32,
    /// Tags.
    pub tags: u32,
    /// Snippets.
    pub snippets: u32,
    /// Forwards.
    pub forwards: u32,
    /// Known hosts.
    pub known_hosts: u32,
}

/// Database handle: one writer `Mutex<Connection>` + read connections. `Send + Sync`.
#[derive(Debug)]
pub struct Store {
    _private: (),
}

impl Store {
    /// Opens (creating + migrating) the database at `path`; WAL, `foreign_keys=ON`, 0600.
    pub fn open(path: &Path) -> Result<Self> {
        Err(StoreError::Unimplemented)
    }
    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self> {
        Err(StoreError::Unimplemented)
    }

    // ---- tree ----
    /// Whole tree, flat, sorted.
    pub fn tree_list(&self) -> Result<Tree> {
        Err(StoreError::Unimplemented)
    }
    /// Creates a folder (appended last among siblings).
    pub fn folder_create(&self, input: FolderInput) -> Result<Folder> {
        Err(StoreError::Unimplemented)
    }
    /// Updates folder fields.
    pub fn folder_update(&self, id: FolderId, patch: FolderPatch) -> Result<Folder> {
        Err(StoreError::Unimplemented)
    }
    /// Moves a folder under `parent`, placed after `after` (`None` = first). Rejects cycles / depth > 8.
    pub fn folder_move(
        &self,
        id: FolderId,
        parent: Option<FolderId>,
        after: Option<NodeRef>,
    ) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes a folder per `mode` (invariant 4).
    pub fn folder_delete(&self, id: FolderId, mode: FolderDeleteMode) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- hosts ----
    /// Creates a host (appended last in its folder).
    pub fn host_create(&self, input: HostInput) -> Result<Host> {
        Err(StoreError::Unimplemented)
    }
    /// Updates host fields.
    pub fn host_update(&self, id: HostId, patch: HostPatch) -> Result<Host> {
        Err(StoreError::Unimplemented)
    }
    /// Moves a host into `folder`, placed after `after` (`None` = first).
    pub fn host_move(
        &self,
        id: HostId,
        folder: Option<FolderId>,
        after: Option<NodeRef>,
    ) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// Copies a host (label suffixed " copy"), placed right after the original.
    pub fn host_duplicate(&self, id: HostId) -> Result<Host> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes hosts (one transaction).
    pub fn host_delete(&self, ids: &[HostId]) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// Resolves inheritance for a host.
    pub fn host_resolve(&self, id: HostId) -> Result<ResolvedHost> {
        Err(StoreError::Unimplemented)
    }
    /// FTS5 search over label, hostname, tags, notes; best match first.
    pub fn host_search(&self, q: &str, limit: u32) -> Result<Vec<HostId>> {
        Err(StoreError::Unimplemented)
    }
    /// Sets `last_connected_at = now`.
    pub fn host_touch_connected(&self, id: HostId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- identities (metadata only) ----
    /// All identities, by label.
    pub fn identity_list(&self) -> Result<Vec<Identity>> {
        Err(StoreError::Unimplemented)
    }
    /// One identity.
    pub fn identity_get(&self, id: IdentityId) -> Result<Identity> {
        Err(StoreError::Unimplemented)
    }
    /// Creates identity metadata (secrets already stored by the vault).
    pub fn identity_create(&self, input: IdentityMetaInput) -> Result<Identity> {
        Err(StoreError::Unimplemented)
    }
    /// Updates; replaced secret rows are deleted in the same transaction.
    pub fn identity_update(&self, id: IdentityId, patch: IdentityMetaPatch) -> Result<Identity> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes identity and its `vault_secret` rows in one transaction (invariant 5).
    pub fn identity_delete(&self, id: IdentityId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- tags ----
    /// All tags by name.
    pub fn tag_list(&self) -> Result<Vec<Tag>> {
        Err(StoreError::Unimplemented)
    }
    /// Creates a tag (`Conflict` on duplicate name).
    pub fn tag_create(&self, input: TagInput) -> Result<Tag> {
        Err(StoreError::Unimplemented)
    }
    /// Updates a tag.
    pub fn tag_update(&self, id: TagId, patch: TagPatch) -> Result<Tag> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes a tag (detaches from hosts).
    pub fn tag_delete(&self, id: TagId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- snippets ----
    /// Snippets; `folder = Some` returns global + that folder's snippets.
    pub fn snippet_list(&self, folder: Option<FolderId>) -> Result<Vec<Snippet>> {
        Err(StoreError::Unimplemented)
    }
    /// Creates a snippet.
    pub fn snippet_create(&self, input: SnippetInput) -> Result<Snippet> {
        Err(StoreError::Unimplemented)
    }
    /// Updates a snippet.
    pub fn snippet_update(&self, id: SnippetId, patch: SnippetPatch) -> Result<Snippet> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes a snippet.
    pub fn snippet_delete(&self, id: SnippetId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- port forwards ----
    /// Forwards; `host = Some` filters by host.
    pub fn forward_list(&self, host: Option<HostId>) -> Result<Vec<PortForward>> {
        Err(StoreError::Unimplemented)
    }
    /// Creates a forward rule.
    pub fn forward_create(&self, input: ForwardInput) -> Result<PortForward> {
        Err(StoreError::Unimplemented)
    }
    /// Updates a forward rule.
    pub fn forward_update(&self, id: ForwardId, patch: ForwardPatch) -> Result<PortForward> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes a forward rule.
    pub fn forward_delete(&self, id: ForwardId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- known hosts ----
    /// All entries for the Known hosts screen.
    pub fn known_host_list(&self) -> Result<Vec<KnownHostSummary>> {
        Err(StoreError::Unimplemented)
    }
    /// Entries matching an exact host pattern (all key types).
    pub fn known_host_lookup(&self, pattern: &str) -> Result<Vec<KnownHost>> {
        Err(StoreError::Unimplemented)
    }
    /// Inserts or replaces on (`host_pattern`, `key_type`).
    pub fn known_host_upsert(&self, kh: KnownHostInput) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes an entry (the only way to clear a changed-key block).
    pub fn known_host_delete(&self, id: KnownHostId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- settings (JSON values) ----
    /// Reads a setting.
    pub fn setting_get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Err(StoreError::Unimplemented)
    }
    /// Writes a setting.
    pub fn setting_set(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- raw secret rows + vault meta (vault crate only) ----
    /// Inserts a secret row.
    pub fn secret_put(&self, row: SecretRow) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// Reads a secret row.
    pub fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>> {
        Err(StoreError::Unimplemented)
    }
    /// Deletes a secret row.
    pub fn secret_delete(&self, id: SecretId) -> Result<()> {
        Err(StoreError::Unimplemented)
    }
    /// All secret ids (backup export).
    pub fn secret_ids(&self) -> Result<Vec<SecretId>> {
        Err(StoreError::Unimplemented)
    }
    /// Reads a `vault_meta` value.
    pub fn vault_meta_get(&self, k: &str) -> Result<Option<Vec<u8>>> {
        Err(StoreError::Unimplemented)
    }
    /// Writes a `vault_meta` value.
    pub fn vault_meta_set(&self, k: &str, v: &[u8]) -> Result<()> {
        Err(StoreError::Unimplemented)
    }

    // ---- backup ----
    /// Consistent snapshot of all non-secret data (single read transaction).
    pub fn export_snapshot(&self) -> Result<Snapshot> {
        Err(StoreError::Unimplemented)
    }
    /// Merges a snapshot (ids kept; existing ids are skipped) in one transaction.
    pub fn import_snapshot(&self, snapshot: Snapshot) -> Result<ImportCounts> {
        Err(StoreError::Unimplemented)
    }
}
