//! Identity metadata, tags, snippets, port forwards, known hosts, secret rows, snapshot.

use serde::{Deserialize, Serialize};

use crate::ids::{
    FolderId, ForwardId, HostId, IdentityId, KnownHostId, SecretId, SnippetId, TagId,
};
use crate::patch::double_option;
use crate::tree::{Folder, Host};

/// Credential kind (`identity.kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    /// Password only.
    Password,
    /// Private key (unencrypted, or passphrase asked on use).
    Key,
    /// Private key + stored passphrase / password fallback.
    KeyPassword,
    /// Use the system SSH agent.
    Agent,
}

/// Identity row. Holds secret *ids* only; deliberately has no `specta::Type` so it can never
/// be returned over IPC (the bridge maps it to `IdentityDto`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Id.
    pub id: IdentityId,
    /// Display label.
    pub label: String,
    /// Optional username; host/folder username wins.
    pub username: Option<String>,
    /// Kind.
    pub kind: IdentityKind,
    /// e.g. `ssh-ed25519`.
    pub key_algo: Option<String>,
    /// `SHA256:<base64>`.
    pub key_fingerprint: Option<String>,
    /// OpenSSH one-line public key.
    pub public_key: Option<String>,
    /// Encrypted password row.
    pub password_secret_id: Option<SecretId>,
    /// Encrypted private key row.
    pub key_secret_id: Option<SecretId>,
    /// Encrypted passphrase row.
    pub passphrase_secret_id: Option<SecretId>,
    /// Unix millis.
    pub created_at: i64,
    /// Unix millis.
    pub updated_at: i64,
}

/// Input for `identity_create` (metadata + already-stored secret ids, set by the bridge).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityMetaInput {
    /// Label.
    pub label: String,
    /// Username.
    pub username: Option<String>,
    /// Kind.
    pub kind: IdentityKind,
    /// Key algorithm.
    pub key_algo: Option<String>,
    /// Fingerprint.
    pub key_fingerprint: Option<String>,
    /// Public key.
    pub public_key: Option<String>,
    /// Password secret.
    pub password_secret_id: Option<SecretId>,
    /// Key secret.
    pub key_secret_id: Option<SecretId>,
    /// Passphrase secret.
    pub passphrase_secret_id: Option<SecretId>,
}

/// Patch for `identity_update`. Replaced secret ids: the old rows are deleted in the same
/// transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdentityMetaPatch {
    /// Label.
    pub label: Option<String>,
    /// Username.
    pub username: Option<Option<String>>,
    /// Kind.
    pub kind: Option<IdentityKind>,
    /// Key algorithm.
    pub key_algo: Option<Option<String>>,
    /// Fingerprint.
    pub key_fingerprint: Option<Option<String>>,
    /// Public key.
    pub public_key: Option<Option<String>>,
    /// Password secret.
    pub password_secret_id: Option<Option<SecretId>>,
    /// Key secret.
    pub key_secret_id: Option<Option<SecretId>>,
    /// Passphrase secret.
    pub passphrase_secret_id: Option<Option<SecretId>>,
}

/// A tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Tag {
    /// Id.
    pub id: TagId,
    /// Unique (case-insensitive) name.
    pub name: String,
    /// Accent token.
    pub color: Option<String>,
}

/// Input for `tag_create`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
pub struct TagInput {
    /// Name.
    pub name: String,
    /// Accent token.
    #[serde(default)]
    pub color: Option<String>,
}

/// Patch for `tag_update`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct TagPatch {
    /// Name.
    pub name: Option<String>,
    /// Accent token.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub color: Option<Option<String>>,
}

/// A command snippet; `folder_id = None` = global.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Snippet {
    /// Id.
    pub id: SnippetId,
    /// Scope folder.
    pub folder_id: Option<FolderId>,
    /// Name.
    pub name: String,
    /// Body sent to the terminal.
    pub body: String,
    /// Fractional index.
    pub sort_key: String,
    /// Unix millis.
    pub created_at: i64,
    /// Unix millis.
    pub updated_at: i64,
}

/// Input for `snippet_create`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
pub struct SnippetInput {
    /// Scope folder.
    #[serde(default)]
    pub folder_id: Option<FolderId>,
    /// Name.
    pub name: String,
    /// Body.
    pub body: String,
}

/// Patch for `snippet_update`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct SnippetPatch {
    /// Scope folder.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<FolderId>, optional)]
    pub folder_id: Option<Option<FolderId>>,
    /// Name.
    pub name: Option<String>,
    /// Body.
    pub body: Option<String>,
}

/// Port-forward kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ForwardKind {
    /// `-L`.
    Local,
    /// `-R`.
    Remote,
    /// `-D` SOCKS5.
    Dynamic,
}

/// A port-forward rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PortForward {
    /// Id.
    pub id: ForwardId,
    /// Owning host.
    pub host_id: HostId,
    /// Kind.
    pub kind: ForwardKind,
    /// Bind address, default `127.0.0.1`.
    pub bind_addr: String,
    /// Bind port.
    pub bind_port: u16,
    /// Destination host (not for dynamic).
    pub dest_host: Option<String>,
    /// Destination port (not for dynamic).
    pub dest_port: Option<u16>,
    /// Start together with the session.
    pub auto_start: bool,
    /// Label.
    pub label: Option<String>,
}

/// Input for `forward_create`.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct ForwardInput {
    /// Owning host.
    pub host_id: HostId,
    /// Kind.
    pub kind: ForwardKind,
    /// Bind address; `None` = `127.0.0.1`.
    #[serde(default)]
    pub bind_addr: Option<String>,
    /// Bind port.
    pub bind_port: u16,
    /// Destination host.
    #[serde(default)]
    pub dest_host: Option<String>,
    /// Destination port.
    #[serde(default)]
    pub dest_port: Option<u16>,
    /// Auto start.
    #[serde(default)]
    pub auto_start: bool,
    /// Label.
    #[serde(default)]
    pub label: Option<String>,
}

/// Patch for `forward_update`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ForwardPatch {
    /// Kind.
    pub kind: Option<ForwardKind>,
    /// Bind address.
    pub bind_addr: Option<String>,
    /// Bind port.
    pub bind_port: Option<u16>,
    /// Destination host.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub dest_host: Option<Option<String>>,
    /// Destination port.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<u16>, optional)]
    pub dest_port: Option<Option<u16>>,
    /// Auto start.
    pub auto_start: Option<bool>,
    /// Label.
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>, optional)]
    pub label: Option<Option<String>>,
}

/// Full known-host row (includes the wire-format key for verification). Not an IPC type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownHost {
    /// Id.
    pub id: KnownHostId,
    /// `[host]:port` OpenSSH form; port 22 → bare host.
    pub host_pattern: String,
    /// e.g. `ssh-ed25519`.
    pub key_type: String,
    /// SSH wire-format public key.
    pub key_blob: Vec<u8>,
    /// `SHA256:<base64>`.
    pub fingerprint: String,
    /// Unix millis.
    pub added_at: i64,
}

/// Known-host entry as shown in the "Known hosts" screen (IPC `KnownHostDto`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct KnownHostSummary {
    /// Id.
    pub id: KnownHostId,
    /// Host pattern.
    pub host_pattern: String,
    /// Key type.
    pub key_type: String,
    /// Fingerprint.
    pub fingerprint: String,
    /// Unix millis.
    pub added_at: i64,
}

/// Input for `known_host_upsert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownHostInput {
    /// Host pattern.
    pub host_pattern: String,
    /// Key type.
    pub key_type: String,
    /// Wire-format key.
    pub key_blob: Vec<u8>,
    /// Fingerprint.
    pub fingerprint: String,
}

/// Raw encrypted secret row, owned by the vault crate.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretRow {
    /// Id (also part of AAD).
    pub id: SecretId,
    /// 24-byte nonce.
    pub nonce: Vec<u8>,
    /// XChaCha20-Poly1305 ciphertext + tag.
    pub ciphertext: Vec<u8>,
    /// Unix millis.
    pub created_at: i64,
}

impl std::fmt::Debug for SecretRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretRow")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Everything except secrets and settings; used by the vault backup (`.tlvault`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Folders.
    pub folders: Vec<Folder>,
    /// Hosts (with `tag_ids`).
    pub hosts: Vec<Host>,
    /// Identities (secret ids refer to rows the vault re-encrypts on import).
    pub identities: Vec<Identity>,
    /// Tags.
    pub tags: Vec<Tag>,
    /// Snippets.
    pub snippets: Vec<Snippet>,
    /// Forwards.
    pub forwards: Vec<PortForward>,
    /// Known hosts.
    pub known_hosts: Vec<KnownHost>,
}
