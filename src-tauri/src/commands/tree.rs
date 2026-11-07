//! Folder / host tree commands (`ipc.md` §store / tree).

use termlite_store::{
    Folder, FolderDeleteMode, FolderId, FolderInput, FolderPatch, Host, HostId, HostInput,
    HostPatch, NodeRef, ResolvedHost, Tree,
};

use crate::api::{ApiError, ApiResult};

/// Whole tree (flat, sorted).
#[tauri::command]
#[specta::specta]
pub async fn tree_list() -> ApiResult<Tree> {
    Err(ApiError::unimplemented())
}

/// Creates a folder (last among siblings).
#[tauri::command]
#[specta::specta]
pub async fn folder_create(input: FolderInput) -> ApiResult<Folder> {
    Err(ApiError::unimplemented())
}

/// Updates folder fields.
#[tauri::command]
#[specta::specta]
pub async fn folder_update(id: FolderId, patch: FolderPatch) -> ApiResult<Folder> {
    Err(ApiError::unimplemented())
}

/// Moves a folder; `after = null` = first. Errors: `Cycle`, `TooDeep`.
#[tauri::command]
#[specta::specta]
pub async fn folder_move(
    id: FolderId,
    parent: Option<FolderId>,
    after: Option<NodeRef>,
) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Deletes a folder.
#[tauri::command]
#[specta::specta]
pub async fn folder_delete(id: FolderId, mode: FolderDeleteMode) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Creates a host (last in folder).
#[tauri::command]
#[specta::specta]
pub async fn host_create(input: HostInput) -> ApiResult<Host> {
    Err(ApiError::unimplemented())
}

/// Updates host fields.
#[tauri::command]
#[specta::specta]
pub async fn host_update(id: HostId, patch: HostPatch) -> ApiResult<Host> {
    Err(ApiError::unimplemented())
}

/// Duplicates a host right after the original.
#[tauri::command]
#[specta::specta]
pub async fn host_duplicate(id: HostId) -> ApiResult<Host> {
    Err(ApiError::unimplemented())
}

/// Deletes hosts (one transaction).
#[tauri::command]
#[specta::specta]
pub async fn host_delete(ids: Vec<HostId>) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Moves a host; `after = null` = first.
#[tauri::command]
#[specta::specta]
pub async fn host_move(
    id: HostId,
    folder: Option<FolderId>,
    after: Option<NodeRef>,
) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Resolves inheritance with per-field source.
#[tauri::command]
#[specta::specta]
pub async fn host_resolve(id: HostId) -> ApiResult<ResolvedHost> {
    Err(ApiError::unimplemented())
}

/// Full-text search; best match first.
#[tauri::command]
#[specta::specta]
pub async fn host_search(q: String, limit: u32) -> ApiResult<Vec<HostId>> {
    Err(ApiError::unimplemented())
}
