//! Identities, tags, snippets, forwards, known hosts, settings (`ipc.md` §store / tree).

use ibc_store::{
    FolderId, ForwardId, ForwardInput, ForwardPatch, HostId, IdentityId, KnownHostId,
    KnownHostSummary, PortForward, Snippet, SnippetId, SnippetInput, SnippetPatch, Tag, TagId,
    TagInput, TagPatch,
};

use crate::api::dto::{IdentityDto, IdentityInput, IdentityPatchInput, SecretInput};
use crate::api::{ApiError, ApiResult};

/// All identities (metadata only).
#[tauri::command]
#[specta::specta]
pub async fn identity_list() -> ApiResult<Vec<IdentityDto>> {
    Err(ApiError::unimplemented())
}

/// Creates an identity; secrets go to the vault. Needs unlocked vault.
#[tauri::command]
#[specta::specta]
pub async fn identity_create(input: IdentityInput) -> ApiResult<IdentityDto> {
    Err(ApiError::unimplemented())
}

/// Updates an identity.
#[tauri::command]
#[specta::specta]
pub async fn identity_update(id: IdentityId, patch: IdentityPatchInput) -> ApiResult<IdentityDto> {
    Err(ApiError::unimplemented())
}

/// Deletes an identity and its secrets.
#[tauri::command]
#[specta::specta]
pub async fn identity_delete(id: IdentityId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Imports a private key file chosen via the dialog plugin.
#[tauri::command]
#[specta::specta]
pub async fn identity_import_file(
    path: String,
    passphrase: Option<SecretInput>,
) -> ApiResult<IdentityDto> {
    Err(ApiError::unimplemented())
}

/// All tags.
#[tauri::command]
#[specta::specta]
pub async fn tag_list() -> ApiResult<Vec<Tag>> {
    Err(ApiError::unimplemented())
}

/// Creates a tag.
#[tauri::command]
#[specta::specta]
pub async fn tag_create(input: TagInput) -> ApiResult<Tag> {
    Err(ApiError::unimplemented())
}

/// Updates a tag.
#[tauri::command]
#[specta::specta]
pub async fn tag_update(id: TagId, patch: TagPatch) -> ApiResult<Tag> {
    Err(ApiError::unimplemented())
}

/// Deletes a tag.
#[tauri::command]
#[specta::specta]
pub async fn tag_delete(id: TagId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Global snippets plus those of `folder_id`.
#[tauri::command]
#[specta::specta]
pub async fn snippet_list(folder_id: Option<FolderId>) -> ApiResult<Vec<Snippet>> {
    Err(ApiError::unimplemented())
}

/// Creates a snippet.
#[tauri::command]
#[specta::specta]
pub async fn snippet_create(input: SnippetInput) -> ApiResult<Snippet> {
    Err(ApiError::unimplemented())
}

/// Updates a snippet.
#[tauri::command]
#[specta::specta]
pub async fn snippet_update(id: SnippetId, patch: SnippetPatch) -> ApiResult<Snippet> {
    Err(ApiError::unimplemented())
}

/// Deletes a snippet.
#[tauri::command]
#[specta::specta]
pub async fn snippet_delete(id: SnippetId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Forward rules (optionally for one host).
#[tauri::command]
#[specta::specta]
pub async fn forward_list(host_id: Option<HostId>) -> ApiResult<Vec<PortForward>> {
    Err(ApiError::unimplemented())
}

/// Creates a forward rule.
#[tauri::command]
#[specta::specta]
pub async fn forward_create(input: ForwardInput) -> ApiResult<PortForward> {
    Err(ApiError::unimplemented())
}

/// Updates a forward rule.
#[tauri::command]
#[specta::specta]
pub async fn forward_update(id: ForwardId, patch: ForwardPatch) -> ApiResult<PortForward> {
    Err(ApiError::unimplemented())
}

/// Deletes a forward rule.
#[tauri::command]
#[specta::specta]
pub async fn forward_delete(id: ForwardId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Known hosts.
#[tauri::command]
#[specta::specta]
pub async fn known_host_list() -> ApiResult<Vec<KnownHostSummary>> {
    Err(ApiError::unimplemented())
}

/// Deletes a known host (only way to clear a changed-key block).
#[tauri::command]
#[specta::specta]
pub async fn known_host_delete(id: KnownHostId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Reads a setting (`null` = unset).
#[tauri::command]
#[specta::specta]
pub async fn settings_get(key: String) -> ApiResult<Option<serde_json::Value>> {
    Err(ApiError::unimplemented())
}

/// Writes a setting.
#[tauri::command]
#[specta::specta]
pub async fn settings_set(key: String, value: serde_json::Value) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}
