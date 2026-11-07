//! SQLite storage: models, migrations, folder tree with inheritance.
//!
//! Contract: `docs/contracts/data-model.md` (FROZEN v1).
// Patch semantics (data-model.md) use `Option<Option<T>>` by contract.
#![allow(clippy::option_option)]
// Public API takes inputs by value per the frozen contract.
#![allow(clippy::needless_pass_by_value)]

mod catalog;
mod db;
mod error;
pub mod flags_list;
mod folders;
mod hosts;
mod identity;
mod ids;
mod known_hosts;
mod misc;
pub mod patch;
mod resolve;
mod resolver;
mod schema;
mod search;
mod siblings;
mod snapshot;
pub mod sort_key;
mod tree;
mod validate;

pub use catalog::{
    ForwardInput, ForwardKind, ForwardPatch, Identity, IdentityKind, IdentityMetaInput,
    IdentityMetaPatch, KnownHost, KnownHostInput, KnownHostSummary, PortForward, SecretRow,
    Snapshot, Snippet, SnippetInput, SnippetPatch, Tag, TagInput, TagPatch,
};
pub use db::Store;
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
