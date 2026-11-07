//! Folder / host tree commands (`ipc.md` §store / tree).

use ibc_shell_store::{
    Folder, FolderDeleteMode, FolderId, FolderInput, FolderPatch, Host, HostId, HostInput,
    HostPatch, NodeRef, ResolvedHost, Tree,
};
use tauri::State;

use crate::api::ApiResult;
use crate::state::AppState;

macro_rules! store_cmd {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty => $call:ident($($pass:expr),*)) => {
        $(#[$doc])*
        #[tauri::command]
        #[specta::specta]
        pub async fn $name(state: State<'_, AppState>, $($arg: $ty),*) -> ApiResult<$ret> {
            Ok(state.store.$call($($pass),*)?)
        }
    };
}

store_cmd!(
    /// Whole tree (flat, sorted).
    tree_list() -> Tree => tree_list()
);
store_cmd!(
    /// Creates a folder (last among siblings).
    folder_create(input: FolderInput) -> Folder => folder_create(input)
);
store_cmd!(
    /// Updates folder fields.
    folder_update(id: FolderId, patch: FolderPatch) -> Folder => folder_update(id, patch)
);
store_cmd!(
    /// Moves a folder; `after = null` = first. Errors: `Cycle`, `TooDeep`.
    folder_move(id: FolderId, parent: Option<FolderId>, after: Option<NodeRef>) -> () => folder_move(id, parent, after)
);
store_cmd!(
    /// Deletes a folder.
    folder_delete(id: FolderId, mode: FolderDeleteMode) -> () => folder_delete(id, mode)
);
store_cmd!(
    /// Creates a host (last in folder).
    host_create(input: HostInput) -> Host => host_create(input)
);
store_cmd!(
    /// Updates host fields.
    host_update(id: HostId, patch: HostPatch) -> Host => host_update(id, patch)
);
store_cmd!(
    /// Duplicates a host right after the original.
    host_duplicate(id: HostId) -> Host => host_duplicate(id)
);
store_cmd!(
    /// Deletes hosts (one transaction).
    host_delete(ids: Vec<HostId>) -> () => host_delete(&ids)
);
store_cmd!(
    /// Moves a host; `after = null` = first.
    host_move(id: HostId, folder: Option<FolderId>, after: Option<NodeRef>) -> () => host_move(id, folder, after)
);
store_cmd!(
    /// Resolves inheritance with per-field source.
    host_resolve(id: HostId) -> ResolvedHost => host_resolve(id)
);
store_cmd!(
    /// Full-text search; best match first.
    host_search(q: String, limit: u32) -> Vec<HostId> => host_search(&q, limit)
);
