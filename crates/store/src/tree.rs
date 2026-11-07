//! Folder / host tree models (IPC DTOs: `FolderDto` = [`Folder`], `HostDto` = [`Host`]).

use serde::{Deserialize, Serialize};

use crate::ids::{FolderId, HostId, IdentityId, TagId};
use crate::patch::double_option;

/// Inheritable connection defaults of a folder (`d_*` columns). `None` = inherit from parent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FolderDefaults {
    /// Default SSH username.
    pub username: Option<String>,
    /// Default port.
    pub port: Option<u16>,
    /// Default identity.
    pub identity_id: Option<IdentityId>,
    /// Default jump host.
    pub jump_host_id: Option<HostId>,
    /// Command sent after the shell opens.
    pub startup_cmd: Option<String>,
    /// Keepalive interval, seconds.
    pub keepalive_s: Option<u32>,
    /// Terminal theme id.
    pub term_theme: Option<String>,
}

/// A folder row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Folder {
    /// Id.
    pub id: FolderId,
    /// Parent folder; `None` = root.
    pub parent_id: Option<FolderId>,
    /// Display name, 1..=128 chars.
    pub name: String,
    /// Lowercase flag code (ISO 3166-1 alpha-2 or circle-flags extra), see `KNOWN_FLAGS`.
    pub country_code: Option<String>,
    /// Accent token name from the UI palette.
    pub color: Option<String>,
    /// Lucide icon name.
    pub icon: Option<String>,
    /// Fractional index among siblings (base-62).
    pub sort_key: String,
    /// Expanded in the sidebar.
    pub expanded: bool,
    /// Inheritable defaults.
    pub defaults: FolderDefaults,
    /// Unix millis.
    pub created_at: i64,
    /// Unix millis.
    pub updated_at: i64,
}

/// Input for `folder_create`. New folder is appended as the last child of `parent_id`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
pub struct FolderInput {
    /// Parent; `None` = root.
    #[serde(default)]
    pub parent_id: Option<FolderId>,
    /// Display name.
    pub name: String,
    /// Flag code.
    #[serde(default)]
    pub country_code: Option<String>,
    /// Accent token.
    #[serde(default)]
    pub color: Option<String>,
    /// Lucide icon name.
    #[serde(default)]
    pub icon: Option<String>,
    /// Inheritable defaults.
    #[serde(default)]
    pub defaults: FolderDefaults,
}

/// Patch for [`FolderDefaults`]; see `patch` module for `Option<Option<T>>` semantics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct FolderDefaultsPatch {
    /// Username.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub username: Option<Option<String>>,
    /// Port.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<u16>, optional)]
    pub port: Option<Option<u16>>,
    /// Identity.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<IdentityId>, optional)]
    pub identity_id: Option<Option<IdentityId>>,
    /// Jump host.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<HostId>, optional)]
    pub jump_host_id: Option<Option<HostId>>,
    /// Startup command.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub startup_cmd: Option<Option<String>>,
    /// Keepalive seconds.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<u32>, optional)]
    pub keepalive_s: Option<Option<u32>>,
    /// Terminal theme.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub term_theme: Option<Option<String>>,
}

/// Patch for `folder_update`. Parent changes go through `folder_move`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct FolderPatch {
    /// New name.
    pub name: Option<String>,
    /// Flag code.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub country_code: Option<Option<String>>,
    /// Accent token.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub color: Option<Option<String>>,
    /// Lucide icon.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub icon: Option<Option<String>>,
    /// Sidebar expansion state.
    pub expanded: Option<bool>,
    /// Defaults patch.
    pub defaults: FolderDefaultsPatch,
}

/// A host row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Host {
    /// Id.
    pub id: HostId,
    /// Containing folder; `None` = root.
    pub folder_id: Option<FolderId>,
    /// Display label, 1..=128 chars.
    pub label: String,
    /// DNS name, IPv4 or IPv6 (no brackets).
    pub hostname: String,
    /// `None` = inherit (final default 22).
    pub port: Option<u16>,
    /// `None` = inherit.
    pub username: Option<String>,
    /// `None` = inherit.
    pub identity_id: Option<IdentityId>,
    /// `None` = inherit.
    pub jump_host_id: Option<HostId>,
    /// `None` = inherit folder flag.
    pub country_code: Option<String>,
    /// `None` = inherit.
    pub startup_cmd: Option<String>,
    /// `None` = inherit (final default 30).
    pub keepalive_s: Option<u32>,
    /// `None` = inherit.
    pub term_theme: Option<String>,
    /// Free-form notes (plain text).
    pub notes: Option<String>,
    /// Attached tags.
    pub tag_ids: Vec<TagId>,
    /// Fractional index among siblings.
    pub sort_key: String,
    /// Unix millis of last successful connect.
    pub last_connected_at: Option<i64>,
    /// Unix millis.
    pub created_at: i64,
    /// Unix millis.
    pub updated_at: i64,
}

/// Input for `host_create`. New host is appended as the last child of `folder_id`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
pub struct HostInput {
    /// Folder; `None` = root.
    #[serde(default)]
    pub folder_id: Option<FolderId>,
    /// Label.
    pub label: String,
    /// Hostname / IP.
    pub hostname: String,
    /// Port.
    #[serde(default)]
    pub port: Option<u16>,
    /// Username.
    #[serde(default)]
    pub username: Option<String>,
    /// Identity.
    #[serde(default)]
    pub identity_id: Option<IdentityId>,
    /// Jump host.
    #[serde(default)]
    pub jump_host_id: Option<HostId>,
    /// Flag code.
    #[serde(default)]
    pub country_code: Option<String>,
    /// Startup command.
    #[serde(default)]
    pub startup_cmd: Option<String>,
    /// Keepalive seconds.
    #[serde(default)]
    pub keepalive_s: Option<u32>,
    /// Terminal theme.
    #[serde(default)]
    pub term_theme: Option<String>,
    /// Notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Tags.
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
}

/// Patch for `host_update`. Folder changes go through `host_move`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct HostPatch {
    /// Label.
    pub label: Option<String>,
    /// Hostname.
    pub hostname: Option<String>,
    /// Port.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<u16>, optional)]
    pub port: Option<Option<u16>>,
    /// Username.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub username: Option<Option<String>>,
    /// Identity.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<IdentityId>, optional)]
    pub identity_id: Option<Option<IdentityId>>,
    /// Jump host.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<HostId>, optional)]
    pub jump_host_id: Option<Option<HostId>>,
    /// Flag code.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub country_code: Option<Option<String>>,
    /// Startup command.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub startup_cmd: Option<Option<String>>,
    /// Keepalive seconds.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<u32>, optional)]
    pub keepalive_s: Option<Option<u32>>,
    /// Terminal theme.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub term_theme: Option<Option<String>>,
    /// Notes.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub notes: Option<Option<String>>,
    /// Replaces the full tag set.
    pub tag_ids: Option<Vec<TagId>>,
}

/// Whole tree, flat. Both lists sorted by (parent, `sort_key`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Tree {
    /// All folders.
    pub folders: Vec<Folder>,
    /// All hosts.
    pub hosts: Vec<Host>,
}

/// Reference to a tree node, used as the "insert after" anchor for moves. TS: `{kind, id}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum NodeRef {
    /// A folder.
    Folder(FolderId),
    /// A host.
    Host(HostId),
}

/// Folder deletion mode (invariant 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum FolderDeleteMode {
    /// Children (folders and hosts) move to the deleted folder's parent.
    MoveToParent,
    /// Children are deleted recursively.
    DeleteContents,
}
