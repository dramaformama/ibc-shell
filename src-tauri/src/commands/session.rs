//! SSH session commands (`ipc.md` §sessions, §Streaming & backpressure).

use std::sync::{Arc, Mutex};

use ibc_store::HostId;
use tauri::ipc::{Channel, InvokeBody, Request};
use tauri::State;
use uuid::Uuid;

use crate::api::dto::{PendingId, QuickConnect, SecretInput, SessionEvent, SessionId};
use crate::api::stream::RawBytes;
use crate::api::{ApiError, ApiResult};
use crate::session_task::spawn_session;
use crate::state::{AppState, Live, SessionHandle, SessionSpec, SessionTarget, ShellCmd};

pub(crate) fn session_of(state: &AppState, id: SessionId) -> ApiResult<Arc<SessionHandle>> {
    state.sessions.get(&id).ok_or(ApiError::NotFound)
}

fn open(
    state: &AppState,
    target: SessionTarget,
    cols: u16,
    rows: u16,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> SessionId {
    let id = SessionId(Uuid::now_v7());
    let handle = Arc::new(SessionHandle {
        id,
        spec: SessionSpec {
            target,
            on_data,
            on_event,
        },
        live: Mutex::new(Live::default()),
    });
    state.sessions.insert(id, handle.clone());
    spawn_session(state, &handle, cols, rows);
    id
}

/// Starts connecting to a stored host. Resolves as soon as the session task is spawned;
/// progress, prompts and failure arrive on `on_event`, terminal output on `on_data`.
#[tauri::command]
#[specta::specta]
pub async fn session_open(
    state: State<'_, AppState>,
    host_id: HostId,
    cols: u16,
    rows: u16,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> ApiResult<SessionId> {
    state.store.host_resolve(host_id)?; // fail fast with NotFound
    Ok(open(
        &state,
        SessionTarget::Host(host_id),
        cols,
        rows,
        on_data,
        on_event,
    ))
}

/// Quick connect without a stored host (`user@host:port`).
#[tauri::command]
#[specta::specta]
pub async fn session_open_adhoc(
    state: State<'_, AppState>,
    target: QuickConnect,
    cols: u16,
    rows: u16,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> ApiResult<SessionId> {
    Ok(open(
        &state,
        SessionTarget::Adhoc(target),
        cols,
        rows,
        on_data,
        on_event,
    ))
}

/// Keystrokes. RAW command (not in generated bindings): body = raw bytes
/// (`InvokeBody::Raw`), header `x-session-id` = session UUID. See ADR-004.
#[tauri::command]
pub async fn shell_write(state: State<'_, AppState>, request: Request<'_>) -> ApiResult<()> {
    let id = request
        .headers()
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(SessionId)
        .ok_or_else(|| ApiError::Validation("missing x-session-id".into()))?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Validation("expected raw body".into()));
    };
    if session_of(&state, id)?.send(ShellCmd::Write(bytes.clone())) {
        Ok(())
    } else {
        Err(ApiError::Cancelled)
    }
}

/// Returns credit for `bytes` written to xterm (called from `write(data, callback)`).
#[tauri::command]
#[specta::specta]
pub async fn shell_ack(
    state: State<'_, AppState>,
    session_id: SessionId,
    bytes: u32,
) -> ApiResult<()> {
    session_of(&state, session_id)?.send(ShellCmd::Ack(bytes));
    Ok(())
}

/// PTY window change.
#[tauri::command]
#[specta::specta]
pub async fn shell_resize(
    state: State<'_, AppState>,
    session_id: SessionId,
    cols: u16,
    rows: u16,
) -> ApiResult<()> {
    if cols == 0 || rows == 0 {
        return Err(ApiError::Validation("size must be > 0".into()));
    }
    session_of(&state, session_id)?.send(ShellCmd::Resize(cols, rows));
    Ok(())
}

/// Closes the session (also cancels any pending prompt of it).
#[tauri::command]
#[specta::specta]
pub async fn session_close(state: State<'_, AppState>, session_id: SessionId) -> ApiResult<()> {
    let handle = state
        .sessions
        .remove(&session_id)
        .ok_or(ApiError::NotFound)?;
    state.pending.cancel_owner(session_id.0);
    handle.send(ShellCmd::Close);
    Ok(())
}

/// Reconnects a disconnected session, reusing its channels.
#[tauri::command]
#[specta::specta]
pub async fn session_reconnect(state: State<'_, AppState>, session_id: SessionId) -> ApiResult<()> {
    let handle = session_of(&state, session_id)?;
    if handle.is_running() {
        return Err(ApiError::Validation("session is still connected".into()));
    }
    let (cols, rows) = handle.live.lock().map_or((80, 24), |l| l.size);
    spawn_session(&state, &handle, cols.max(1), rows.max(1));
    Ok(())
}

/// Accepts an UNKNOWN host key (TOFU). There is no command to accept a changed key.
#[tauri::command]
#[specta::specta]
pub async fn hostkey_accept(
    state: State<'_, AppState>,
    pending_id: PendingId,
    remember: bool,
) -> ApiResult<()> {
    state.pending.reply_host_key(pending_id, remember)
}

/// keyboard-interactive answers (same order as prompts); `null` = cancel.
#[tauri::command]
#[specta::specta]
pub async fn kbd_interactive_reply(
    state: State<'_, AppState>,
    pending_id: PendingId,
    answers: Option<Vec<SecretInput>>,
) -> ApiResult<()> {
    state.pending.reply_kbd(
        pending_id,
        answers.map(|a| a.into_iter().map(|s| s.0).collect()),
    )
}

/// Key passphrase; `null` = cancel.
#[tauri::command]
#[specta::specta]
pub async fn passphrase_reply(
    state: State<'_, AppState>,
    pending_id: PendingId,
    passphrase: Option<SecretInput>,
) -> ApiResult<()> {
    state
        .pending
        .reply_passphrase(pending_id, passphrase.map(|s| s.0))
}
