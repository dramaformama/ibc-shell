//! Port forwards of terminal sessions: start / stop, auto-start after connecting, stop with
//! the connection. State changes go to the session's event channel as `ForwardStatus`.

use std::sync::PoisonError;

use ibc_shell_ssh::ForwardSpec;
use ibc_shell_store::{ForwardId, ForwardKind, HostId, PortForward};
use tauri::State;

use crate::api::dto::{SessionEvent, SessionId};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::state::{AppState, SessionHandle, SessionTarget};
use ibc_shell_ssh::ForwardState;

fn status(
    handle: &SessionHandle,
    forward_id: ForwardId,
    state: ForwardState,
    error: Option<String>,
) {
    let _ = handle.spec.on_event.send(SessionEvent::ForwardStatus {
        forward_id,
        state,
        error,
    });
}

/// Turns a stored rule into what the SSH layer runs.
pub(crate) fn spec_of(f: &PortForward) -> ApiResult<ForwardSpec> {
    let bind_addr = f.bind_addr.clone();
    let dest = || match (&f.dest_host, f.dest_port) {
        (Some(h), Some(p)) if !h.trim().is_empty() && p != 0 => Ok((h.trim().to_owned(), p)),
        _ => Err(ApiError::Validation("forward needs a destination".into())),
    };
    Ok(match f.kind {
        ForwardKind::Local => {
            let (dest_host, dest_port) = dest()?;
            ForwardSpec::Local {
                bind_addr,
                bind_port: f.bind_port,
                dest_host,
                dest_port,
            }
        }
        ForwardKind::Remote => {
            let (dest_host, dest_port) = dest()?;
            ForwardSpec::Remote {
                bind_addr,
                bind_port: f.bind_port,
                dest_host,
                dest_port,
            }
        }
        ForwardKind::Dynamic => ForwardSpec::Dynamic {
            bind_addr,
            bind_port: f.bind_port,
        },
    })
}

fn host_of(handle: &SessionHandle) -> ApiResult<HostId> {
    match handle.spec.target {
        SessionTarget::Host(id) => Ok(id),
        _ => Err(ApiError::Validation("forwards need a saved host".into())),
    }
}

/// Starts one rule in a connected session; returns the port actually bound.
pub(crate) async fn start(handle: &SessionHandle, f: &PortForward) -> ApiResult<u16> {
    let conn = {
        let live = handle.live.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(running) = live.forwards.get(&f.id) {
            return Ok(running.bound_port());
        }
        live.conn
            .clone()
            .ok_or_else(|| ApiError::Validation("session is not connected".into()))?
    };
    let spec = spec_of(f)?;
    match conn.start_forward(spec).await {
        Ok(fwd) => {
            let port = fwd.bound_port();
            let mut live = handle.live.lock().unwrap_or_else(PoisonError::into_inner);
            // A parallel start of the same rule won: keep that one, this one stops on drop.
            if live.forwards.contains_key(&f.id) {
                return Ok(live
                    .forwards
                    .get(&f.id)
                    .map_or(port, ibc_shell_ssh::ForwardHandle::bound_port));
            }
            live.forwards.insert(f.id, fwd);
            drop(live);
            status(handle, f.id, ForwardState::Active, None);
            Ok(port)
        }
        Err(e) => {
            status(handle, f.id, ForwardState::Failed, Some(e.to_string()));
            Err(e.into())
        }
    }
}

/// Stops one rule; `false` when it was not running.
pub(crate) fn stop(handle: &SessionHandle, id: ForwardId) -> bool {
    let fwd = handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forwards
        .remove(&id);
    let was = fwd.is_some();
    drop(fwd);
    if was {
        status(handle, id, ForwardState::Stopped, None);
    }
    was
}

/// Stops every forward of the session (the connection ended or is ending).
pub(crate) fn stop_all(handle: &SessionHandle) {
    let all = std::mem::take(
        &mut handle
            .live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .forwards,
    );
    for (id, fwd) in all {
        drop(fwd);
        status(handle, id, ForwardState::Stopped, None);
    }
}

/// Stops a rule in every session (it is being deleted).
pub(crate) fn stop_everywhere(state: &AppState, id: ForwardId) {
    for handle in state.sessions.values() {
        stop(&handle, id);
    }
}

/// Starts the host's `auto_start` rules; failures are reported as `ForwardStatus` events.
pub(crate) async fn auto_start(state: &AppState, handle: &SessionHandle) {
    let Ok(host) = host_of(handle) else { return };
    let Ok(rules) = state.store.forward_list(Some(host)) else {
        return;
    };
    for f in rules.iter().filter(|f| f.auto_start) {
        let _ = start(handle, f).await;
    }
}

/// Starts a forward rule of the session's host; returns the bound port.
#[tauri::command]
#[specta::specta]
pub async fn forward_start(
    state: State<'_, AppState>,
    session_id: SessionId,
    forward_id: ForwardId,
) -> ApiResult<u16> {
    let handle = session_of(&state, session_id)?;
    let host = host_of(&handle)?;
    let rule = state
        .store
        .forward_list(Some(host))?
        .into_iter()
        .find(|f| f.id == forward_id)
        .ok_or(ApiError::NotFound)?;
    start(&handle, &rule).await
}

/// Stops a running forward of the session (no-op when it is not running).
#[tauri::command]
#[specta::specta]
pub async fn forward_stop(
    state: State<'_, AppState>,
    session_id: SessionId,
    forward_id: ForwardId,
) -> ApiResult<()> {
    let handle = session_of(&state, session_id)?;
    stop(&handle, forward_id);
    Ok(())
}

/// Running forwards of the session with their bound ports.
#[tauri::command]
#[specta::specta]
pub async fn forward_active(
    state: State<'_, AppState>,
    session_id: SessionId,
) -> ApiResult<Vec<ActiveForward>> {
    let handle = session_of(&state, session_id)?;
    let live = handle.live.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(live
        .forwards
        .iter()
        .map(|(id, f)| ActiveForward {
            forward_id: *id,
            port: f.bound_port(),
        })
        .collect())
}

/// A running forward: rule id + the port actually bound.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct ActiveForward {
    /// Rule.
    pub forward_id: ForwardId,
    /// Bound port (differs from the rule when it asked for 0).
    pub port: u16,
}
