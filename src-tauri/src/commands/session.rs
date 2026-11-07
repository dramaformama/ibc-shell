//! SSH session commands (`ipc.md` §sessions, §Streaming & backpressure).

use tauri::ipc::{Channel, Request};
use termlite_store::HostId;

use crate::api::dto::{PendingId, QuickConnect, SecretInput, SessionEvent, SessionId};
use crate::api::stream::RawBytes;
use crate::api::{ApiError, ApiResult};

/// Starts connecting to a stored host. Resolves as soon as the session task is spawned;
/// progress, prompts and failure arrive on `on_event`, terminal output on `on_data`.
#[tauri::command]
#[specta::specta]
pub async fn session_open(
    host_id: HostId,
    cols: u16,
    rows: u16,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> ApiResult<SessionId> {
    Err(ApiError::unimplemented())
}

/// Quick connect without a stored host (`user@host:port`).
#[tauri::command]
#[specta::specta]
pub async fn session_open_adhoc(
    target: QuickConnect,
    cols: u16,
    rows: u16,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> ApiResult<SessionId> {
    Err(ApiError::unimplemented())
}

/// Keystrokes. RAW command (not in generated bindings): body = raw bytes
/// (`InvokeBody::Raw`), header `x-session-id` = session UUID. See ADR-004.
#[tauri::command]
pub async fn shell_write(request: Request<'_>) -> ApiResult<()> {
    let _ = request.headers().get("x-session-id");
    Err(ApiError::unimplemented())
}

/// Returns credit for `bytes` written to xterm (called from `write(data, callback)`).
#[tauri::command]
#[specta::specta]
pub async fn shell_ack(session_id: SessionId, bytes: u32) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// PTY window change.
#[tauri::command]
#[specta::specta]
pub async fn shell_resize(session_id: SessionId, cols: u16, rows: u16) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Closes the session (also cancels any pending prompt of it).
#[tauri::command]
#[specta::specta]
pub async fn session_close(session_id: SessionId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Reconnects a disconnected session, reusing its channels.
#[tauri::command]
#[specta::specta]
pub async fn session_reconnect(session_id: SessionId) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Accepts an UNKNOWN host key (TOFU). There is no command to accept a changed key.
#[tauri::command]
#[specta::specta]
pub async fn hostkey_accept(pending_id: PendingId, remember: bool) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// keyboard-interactive answers (same order as prompts); `null` = cancel.
#[tauri::command]
#[specta::specta]
pub async fn kbd_interactive_reply(
    pending_id: PendingId,
    answers: Option<Vec<SecretInput>>,
) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Key passphrase; `null` = cancel.
#[tauri::command]
#[specta::specta]
pub async fn passphrase_reply(
    pending_id: PendingId,
    passphrase: Option<SecretInput>,
) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}
