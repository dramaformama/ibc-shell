//! Host metrics command (`ipc.md` §Stats, ADR-009).

use tauri::State;

use crate::api::dto::{HostStats, SessionId};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::state::AppState;

/// One-shot sample of the remote Linux host over the session's existing SSH connection
/// (exec channel, independent of the shell). Takes ~1 s. Passive for auto-lock.
#[tauri::command]
#[specta::specta]
pub async fn stats_sample(
    state: State<'_, AppState>,
    session_id: SessionId,
) -> ApiResult<HostStats> {
    let conn = {
        let h = session_of(&state, session_id)?;
        let live = h
            .live
            .lock()
            .map_err(|_| ApiError::Internal("state poisoned".into()))?;
        live.conn
            .clone()
            .ok_or(ApiError::Network("session not connected".into()))?
    };
    Ok(ibc_shell_ssh::sample_stats(&conn).await?)
}
