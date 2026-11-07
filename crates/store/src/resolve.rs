//! Inheritance resolution result types (`host_resolve`).

use serde::{Deserialize, Serialize};

use crate::ids::{FolderId, HostId, IdentityId};

/// Where a resolved value came from. TS: `{kind:"host"} | {kind:"folder", id} | {kind:"default"}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum Source {
    /// The host's own field.
    Host,
    /// Nearest ancestor folder with a non-NULL value.
    Folder(FolderId),
    /// Built-in default (port 22, keepalive 30, otherwise none). For `username` also used when
    /// the value came from the identity.
    Default,
}

/// A value plus its source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Resolved<T> {
    /// Effective value.
    pub value: T,
    /// Origin.
    pub source: Source,
}

/// Fully resolved host (IPC `ResolvedHostDto`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResolvedHost {
    /// Host id.
    pub id: HostId,
    /// Label.
    pub label: String,
    /// Hostname.
    pub hostname: String,
    /// Default 22.
    pub port: Resolved<u16>,
    /// host → folder `d_username` → identity.username → `None` (UI prompts).
    pub username: Resolved<Option<String>>,
    /// Identity.
    pub identity_id: Resolved<Option<IdentityId>>,
    /// Jump chain, outermost first, max 3 hops, already resolved.
    pub jump_chain: Vec<HostId>,
    /// Flag code.
    pub country_code: Resolved<Option<String>>,
    /// Startup command.
    pub startup_cmd: Resolved<Option<String>>,
    /// Default 30.
    pub keepalive_s: Resolved<u32>,
    /// Terminal theme.
    pub term_theme: Resolved<Option<String>>,
}
