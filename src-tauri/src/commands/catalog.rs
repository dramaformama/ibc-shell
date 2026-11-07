//! Tags, snippets, forwards, known hosts, settings (`ipc.md` §store / tree).

use ibc_store::{
    FolderId, ForwardId, ForwardInput, ForwardPatch, HostId, KnownHostId, KnownHostSummary,
    PortForward, Snippet, SnippetId, SnippetInput, SnippetPatch, Tag, TagId, TagInput, TagPatch,
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
    /// All tags.
    tag_list() -> Vec<Tag> => tag_list()
);
store_cmd!(
    /// Creates a tag.
    tag_create(input: TagInput) -> Tag => tag_create(input)
);
store_cmd!(
    /// Updates a tag.
    tag_update(id: TagId, patch: TagPatch) -> Tag => tag_update(id, patch)
);
store_cmd!(
    /// Deletes a tag.
    tag_delete(id: TagId) -> () => tag_delete(id)
);
store_cmd!(
    /// Global snippets plus those of `folder_id`.
    snippet_list(folder_id: Option<FolderId>) -> Vec<Snippet> => snippet_list(folder_id)
);
store_cmd!(
    /// Creates a snippet.
    snippet_create(input: SnippetInput) -> Snippet => snippet_create(input)
);
store_cmd!(
    /// Updates a snippet.
    snippet_update(id: SnippetId, patch: SnippetPatch) -> Snippet => snippet_update(id, patch)
);
store_cmd!(
    /// Deletes a snippet.
    snippet_delete(id: SnippetId) -> () => snippet_delete(id)
);
store_cmd!(
    /// Forward rules of a host (or all).
    forward_list(host_id: Option<HostId>) -> Vec<PortForward> => forward_list(host_id)
);
store_cmd!(
    /// Creates a forward rule.
    forward_create(input: ForwardInput) -> PortForward => forward_create(input)
);
store_cmd!(
    /// Updates a forward rule.
    forward_update(id: ForwardId, patch: ForwardPatch) -> PortForward => forward_update(id, patch)
);
store_cmd!(
    /// Deletes a forward rule.
    forward_delete(id: ForwardId) -> () => forward_delete(id)
);
store_cmd!(
    /// Known hosts for the management screen (no key blobs).
    known_host_list() -> Vec<KnownHostSummary> => known_host_list()
);
store_cmd!(
    /// Deletes a known host (the only way to clear a changed-key block).
    known_host_delete(id: KnownHostId) -> () => known_host_delete(id)
);
store_cmd!(
    /// Reads a UI setting.
    settings_get(key: String) -> Option<serde_json::Value> => setting_get(&key)
);
/// Writes a UI setting; `security.autolock_minutes` is applied to the vault at once (ADR-007).
#[tauri::command]
#[specta::specta]
pub async fn settings_set(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> ApiResult<()> {
    state.store.setting_set(&key, &value)?;
    if key == crate::autolock::AUTOLOCK_KEY {
        crate::autolock::apply_from_store(&state.store, &state.vault);
    }
    Ok(())
}
