//! SFTP, local FS and transfer commands (`ipc.md` §SFTP).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ibc_shell_sftp::{
    local, ConflictChoice, ConflictId, DirListing, Endpoint as EngineEndpoint, Entry, Sftp,
    TransferEvent, TransferId, TransferJob,
};
use tauri::ipc::Channel;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::api::dto::{Endpoint, SessionEvent, SftpId, SftpTarget, TransferIds, TransferRequest};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::connect::{build_chain, StoreKnownHosts};
use crate::prompts::EventPrompter;
use crate::state::{AppState, SessionTarget, SftpHandle};

pub(super) fn sftp_of(state: &AppState, id: SftpId) -> ApiResult<Arc<Sftp>> {
    state
        .sftps
        .get(&id)
        .map(|h| h.sftp.clone())
        .ok_or(ApiError::NotFound)
}

/// Opens SFTP over an existing session or a new connection (prompts → `on_event`).
#[tauri::command]
#[specta::specta]
pub async fn sftp_open(
    window: WebviewWindow,
    state: State<'_, AppState>,
    target: SftpTarget,
    on_event: Channel<SessionEvent>,
) -> ApiResult<SftpId> {
    let id = SftpId(Uuid::now_v7());
    let (conn, host) = match target {
        SftpTarget::Session { session_id } => {
            let h = session_of(&state, session_id)?;
            let live = h
                .live
                .lock()
                .map_err(|_| ApiError::Internal("state poisoned".into()))?;
            let host = match h.spec.target {
                SessionTarget::Host(id) => Some(id),
                _ => None,
            };
            (
                live.conn
                    .clone()
                    .ok_or(ApiError::Network("session not connected".into()))?,
                host,
            )
        }
        SftpTarget::Host { host_id } => {
            let chain = build_chain(&state.store, &state.vault, &SessionTarget::Host(host_id))?;
            let prompter = EventPrompter {
                owner: id.0,
                events: crate::session_output::Relay::new(on_event.clone()),
                pending: state.pending.clone(),
            };
            let known = StoreKnownHosts(state.store.clone());
            match ibc_shell_ssh::connect(chain, &known, &prompter).await {
                Ok(c) => (Arc::new(c), Some(host_id)),
                Err(e) => {
                    let err = ApiError::from(e);
                    let _ = on_event.send(SessionEvent::Failed { error: err.clone() });
                    return Err(err);
                }
            }
        }
    };
    let sftp = match Sftp::open(&conn).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            let err = ApiError::from(e);
            let _ = on_event.send(SessionEvent::Failed { error: err.clone() });
            return Err(err);
        }
    };
    state.sftps.insert(
        id,
        Arc::new(SftpHandle {
            owner: std::sync::Mutex::new(window.label().into()),
            host,
            sftp,
            conn: conn.clone(),
        }),
    );
    if window.get_webview_window(window.label()).is_none() {
        if let Some(handle) = state.sftps.remove(&id) {
            handle.sftp.close().await;
        }
        return Err(ApiError::Cancelled);
    }
    let _ = on_event.send(SessionEvent::Connected {
        server_banner: conn.server_banner().map(str::to_owned),
    });
    Ok(id)
}

/// Closes an SFTP session (transfers using it fail with `Cancelled`).
#[tauri::command]
#[specta::specta]
pub async fn sftp_close(
    window: WebviewWindow,
    state: State<'_, AppState>,
    sftp_id: SftpId,
) -> ApiResult<()> {
    let h = state
        .sftps
        .remove_if(&sftp_id, |handle| {
            handle
                .owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_str()
                == window.label()
        })
        .ok_or(ApiError::NotFound)?;
    state.pending.cancel_owner(sftp_id.0);
    h.sftp.close().await;
    Ok(())
}

/// Lists a remote directory.
#[tauri::command]
#[specta::specta]
pub async fn sftp_list(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
) -> ApiResult<DirListing> {
    Ok(sftp_of(&state, sftp_id)?.list(&path).await?)
}

/// `lstat`.
#[tauri::command]
#[specta::specta]
pub async fn sftp_stat(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
) -> ApiResult<Entry> {
    Ok(sftp_of(&state, sftp_id)?.stat(&path).await?)
}

/// `mkdir`.
#[tauri::command]
#[specta::specta]
pub async fn sftp_mkdir(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
) -> ApiResult<()> {
    Ok(sftp_of(&state, sftp_id)?.mkdir(&path).await?)
}

/// Rename / move.
#[tauri::command]
#[specta::specta]
pub async fn sftp_rename(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    from: String,
    to: String,
) -> ApiResult<()> {
    Ok(sftp_of(&state, sftp_id)?.rename(&from, &to).await?)
}

/// Remove a file, or a tree when `recursive`.
#[tauri::command]
#[specta::specta]
pub async fn sftp_remove(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
    recursive: bool,
) -> ApiResult<()> {
    Ok(sftp_of(&state, sftp_id)?.remove(&path, recursive).await?)
}

/// Change permissions.
#[tauri::command]
#[specta::specta]
pub async fn sftp_chmod(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
    mode: u32,
) -> ApiResult<()> {
    Ok(sftp_of(&state, sftp_id)?.chmod(&path, mode).await?)
}

/// Canonical absolute path (`"."` → remote home).
#[tauri::command]
#[specta::specta]
pub async fn sftp_realpath(
    state: State<'_, AppState>,
    sftp_id: SftpId,
    path: String,
) -> ApiResult<String> {
    Ok(sftp_of(&state, sftp_id)?.realpath(&path).await?)
}

// ---- local FS -------------------------------------------------------------------------

fn desktop_only() -> ApiResult<()> {
    if cfg!(mobile) {
        Err(ApiError::Unsupported)
    } else {
        Ok(())
    }
}

/// Validates a directory that must exist (listing): under an allowed root, or an ancestor of one.
pub(crate) fn local_dir(state: &AppState, path: &str) -> ApiResult<PathBuf> {
    desktop_only()?;
    let p = Path::new(path);
    if !p.is_absolute() || !state.roots.allows(p, true) {
        return Err(ApiError::Validation("local path not allowed".into()));
    }
    std::fs::canonicalize(p).map_err(|_| ApiError::NotFound)
}

/// Validates an entry that may not exist / may be a symlink (`lstat` semantics): its parent
/// must be under an allowed root (not merely an ancestor); returns `parent/name`.
pub(crate) fn local_leaf(state: &AppState, path: &str) -> ApiResult<PathBuf> {
    desktop_only()?;
    let p = Path::new(path);
    let (Some(parent), Some(name)) = (p.parent(), p.file_name()) else {
        return Err(ApiError::Validation("local path not allowed".into()));
    };
    if !p.is_absolute() || !state.roots.allows(parent, false) {
        return Err(ApiError::Validation("local path not allowed".into()));
    }
    let parent = std::fs::canonicalize(parent).map_err(|_| ApiError::NotFound)?;
    Ok(parent.join(name))
}

/// Local home directory; registers it as an allowed root. Desktop only (`Unsupported` on mobile).
#[tauri::command]
#[specta::specta]
pub async fn local_home(state: State<'_, AppState>) -> ApiResult<String> {
    desktop_only()?;
    let home = local::home()?;
    state.roots.add(Path::new(&home));
    Ok(home)
}

/// Lists a local directory (must be under an allowed root).
#[tauri::command]
#[specta::specta]
pub async fn local_list(state: State<'_, AppState>, path: String) -> ApiResult<DirListing> {
    let dir = local_dir(&state, &path)?;
    let listing = tauri::async_runtime::spawn_blocking(move || local::list(&dir))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))??;
    // Listed directories become navigable roots (ipc.md Rules).
    state.roots.add(Path::new(&listing.path));
    Ok(listing)
}

/// Creates a local directory.
#[tauri::command]
#[specta::specta]
pub async fn local_mkdir(state: State<'_, AppState>, path: String) -> ApiResult<()> {
    Ok(local::mkdir(&local_leaf(&state, &path)?)?)
}

/// Renames a local entry.
#[tauri::command]
#[specta::specta]
pub async fn local_rename(state: State<'_, AppState>, from: String, to: String) -> ApiResult<()> {
    let (f, t) = (local_leaf(&state, &from)?, local_leaf(&state, &to)?);
    Ok(local::rename(&f, &t)?)
}

/// Removes a local entry.
#[tauri::command]
#[specta::specta]
pub async fn local_remove(
    state: State<'_, AppState>,
    path: String,
    recursive: bool,
) -> ApiResult<()> {
    let p = local_leaf(&state, &path)?;
    tauri::async_runtime::spawn_blocking(move || local::remove(&p, recursive))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))??;
    Ok(())
}

// ---- transfers ------------------------------------------------------------------------

fn engine_endpoint(state: &AppState, e: Endpoint) -> ApiResult<EngineEndpoint> {
    match e {
        Endpoint::Local => desktop_only().map(|()| EngineEndpoint::Local),
        Endpoint::Remote { sftp_id } => Ok(EngineEndpoint::Remote(sftp_of(state, sftp_id)?)),
    }
}

/// Queues transfers; one id per item.
#[tauri::command]
#[specta::specta]
pub async fn transfer_enqueue(
    state: State<'_, AppState>,
    request: TransferRequest,
) -> ApiResult<TransferIds> {
    if request.items.is_empty() {
        return Err(ApiError::Validation("no items".into()));
    }
    let src = engine_endpoint(&state, request.src)?;
    let dst = engine_endpoint(&state, request.dst)?;
    let mut items = request.items;
    let mut dst_dir = request.dst_dir;
    if request.src == Endpoint::Local {
        items = items
            .iter()
            .map(|i| local_leaf(&state, i).map(|p| p.to_string_lossy().into_owned()))
            .collect::<ApiResult<_>>()?;
    }
    if request.dst == Endpoint::Local {
        // A listed directory is itself a root (including a volume root or the user's home).
        // Validate the directory, not its parent, and disallow unlisted ancestors.
        if !state.roots.allows(Path::new(&dst_dir), false) {
            return Err(ApiError::Validation("local path not allowed".into()));
        }
        dst_dir = local_dir(&state, &dst_dir)?.to_string_lossy().into_owned();
    }
    let job = TransferJob {
        src,
        dst,
        items,
        dst_dir,
        conflict: request.conflict,
    };
    Ok(state.transfers.engine.enqueue(job)?)
}

/// Pauses a transfer.
#[tauri::command]
#[specta::specta]
pub async fn transfer_pause(state: State<'_, AppState>, id: TransferId) -> ApiResult<()> {
    Ok(state.transfers.engine.pause(id)?)
}

/// Resumes a transfer.
#[tauri::command]
#[specta::specta]
pub async fn transfer_resume(state: State<'_, AppState>, id: TransferId) -> ApiResult<()> {
    Ok(state.transfers.engine.resume(id)?)
}

/// Cancels a transfer.
#[tauri::command]
#[specta::specta]
pub async fn transfer_cancel(state: State<'_, AppState>, id: TransferId) -> ApiResult<()> {
    Ok(state.transfers.engine.cancel(id)?)
}

/// Retries a failed / cancelled transfer.
#[tauri::command]
#[specta::specta]
pub async fn transfer_retry(state: State<'_, AppState>, id: TransferId) -> ApiResult<()> {
    Ok(state.transfers.engine.retry(id)?)
}

/// Answers a `TransferEvent::Conflict`.
#[tauri::command]
#[specta::specta]
pub async fn transfer_conflict_reply(
    state: State<'_, AppState>,
    pending_id: ConflictId,
    choice: ConflictChoice,
    apply_to_all: bool,
) -> ApiResult<()> {
    Ok(state
        .transfers
        .engine
        .conflict_reply(pending_id, choice, apply_to_all)?)
}

/// Subscribes to all transfer events (replaces the previous subscriber).
#[tauri::command]
#[specta::specta]
pub async fn transfers_subscribe(
    window: WebviewWindow,
    state: State<'_, AppState>,
    on_event: Channel<TransferEvent>,
) -> ApiResult<()> {
    state.transfers.set_subscriber(window.label(), on_event);
    if window.get_webview_window(window.label()).is_none() {
        state.transfers.remove_subscriber(window.label());
        return Err(ApiError::Cancelled);
    }
    Ok(())
}
