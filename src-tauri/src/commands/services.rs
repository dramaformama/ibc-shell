//! Server services over the session's connection (`ipc.md` §Services, ADR-012): Docker
//! containers (list, logs, start / stop / restart) and TLS certificate expiry.

use tauri::State;

use crate::api::dto::{CertReport, DockerAction, DockerReport, SessionId};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::commands::vault::require_unlocked;
use crate::state::AppState;

/// The live connection of a session.
pub(crate) fn conn_of(
    state: &AppState,
    id: SessionId,
) -> ApiResult<std::sync::Arc<ibc_shell_ssh::Connection>> {
    let h = session_of(state, id)?;
    let live = h
        .live
        .lock()
        .map_err(|_| ApiError::Internal("state poisoned".into()))?;
    live.conn
        .clone()
        .ok_or(ApiError::Network("session not connected".into()))
}

/// Docker containers with CPU / memory. Read-only; passive for auto-lock.
#[tauri::command]
#[specta::specta]
pub async fn docker_list(
    state: State<'_, AppState>,
    session_id: SessionId,
) -> ApiResult<DockerReport> {
    let conn = conn_of(&state, session_id)?;
    Ok(ibc_shell_ssh::docker_list(&conn).await?)
}

/// Last `lines` (20–2000) log lines of a container. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn docker_logs(
    state: State<'_, AppState>,
    session_id: SessionId,
    container_id: String,
    lines: u32,
) -> ApiResult<String> {
    require_unlocked(&state)?;
    let conn = conn_of(&state, session_id)?;
    ibc_shell_ssh::docker_logs(&conn, &container_id, lines)
        .await
        .map_err(docker_err)
}

/// Starts, stops or restarts one container (the UI asks the user first).
#[tauri::command]
#[specta::specta]
pub async fn docker_action(
    state: State<'_, AppState>,
    session_id: SessionId,
    container_id: String,
    action: DockerAction,
) -> ApiResult<()> {
    require_unlocked(&state)?;
    let conn = conn_of(&state, session_id)?;
    ibc_shell_ssh::docker_act(&conn, &container_id, action)
        .await
        .map_err(docker_err)
}

/// TLS certificates found on the server with their expiry. Read-only; passive for auto-lock.
#[tauri::command]
#[specta::specta]
pub async fn cert_list(state: State<'_, AppState>, session_id: SessionId) -> ApiResult<CertReport> {
    let conn = conn_of(&state, session_id)?;
    Ok(ibc_shell_ssh::cert_list(&conn).await?)
}

/// The CLI's own error line becomes `Validation` so the UI can show it as is.
fn docker_err(e: ibc_shell_ssh::SshError) -> ApiError {
    match e {
        ibc_shell_ssh::SshError::Protocol(s) => ApiError::Validation(s),
        other => other.into(),
    }
}
