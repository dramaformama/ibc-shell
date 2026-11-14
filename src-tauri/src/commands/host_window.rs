//! Opens an independent desktop window for a saved host (ADR-019).

use ibc_shell_store::HostId;
use serde::Deserialize;
use tauri::{State, WebviewWindow};

use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

/// Initial view of a saved host in a new window.
#[derive(Clone, Copy, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum HostWindowView {
    Terminal,
    Status,
    Sftp,
}

/// Creates a guarded window with an independent connection.
#[tauri::command]
#[specta::specta]
pub(crate) async fn host_window_open(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host_id: HostId,
    view: HostWindowView,
    transfer: Option<HostWindowTransfer>,
    handoffs: State<'_, Handoffs>,
) -> ApiResult<String> {
    if window.label() != "main" {
        return Err(ApiError::Unsupported);
    }
    if state.vault.state()? != ibc_shell_vault::VaultState::Unlocked {
        return Err(ApiError::VaultLocked);
    }

    #[cfg(desktop)]
    {
        use tauri::Manager;
        let host = state.store.host_resolve(host_id)?;
        let view = match view {
            HostWindowView::Terminal => "terminal",
            HostWindowView::Status => "status",
            HostWindowView::Sftp => "sftp",
        };
        let label = format!("host-{}", uuid::Uuid::now_v7());
        if let Some(payload) = transfer {
            if payload
                .screen
                .as_ref()
                .is_some_and(|screen| screen.len() > 16 * 1024 * 1024)
            {
                return Err(ApiError::Validation("terminal snapshot too large".into()));
            }
            if let Some(id) = payload.terminal {
                let session = state.sessions.get(&id).ok_or(ApiError::NotFound)?;
                if !matches!(session.spec.target, SessionTarget::Host(id) if id == host_id)
                    || session
                        .owner
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .as_str()
                        != window.label()
                {
                    return Err(ApiError::NotFound);
                }
            }
            if let Some(id) = payload.files {
                let files = state.sftps.get(&id).ok_or(ApiError::NotFound)?;
                if files.host != Some(host_id)
                    || files
                        .owner
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .as_str()
                        != window.label()
                {
                    return Err(ApiError::NotFound);
                }
            }
            handoffs
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(
                    label.clone(),
                    Handoff {
                        cancelled: false,
                        source: window.label().into(),
                        payload: Some(payload),
                    },
                );
        }
        let path = format!(
            "index.html?host={}&view={view}&handoff={}",
            host_id.0,
            handoffs
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .contains_key(&label)
        );
        if let Err(error) =
            crate::window::create_host(window.app_handle(), label.clone(), path, host.label)
        {
            handoffs.close(&label, &state);
            return Err(error);
        }
        Ok(label)
    }
    #[cfg(not(desktop))]
    {
        let _ = (host_id, view, transfer, handoffs);
        Err(ApiError::Unsupported)
    }
}

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use tauri::ipc::Channel;

use crate::api::{
    dto::{SessionEvent, SessionId, SftpId},
    stream::RawBytes,
};
use crate::state::SessionTarget;

#[derive(Clone, Deserialize, serde::Serialize, specta::Type)]
pub(crate) struct HostWindowTransfer {
    terminal: Option<SessionId>,
    files: Option<SftpId>,
    screen: Option<String>,
    cwd: Option<String>,
    left: Option<String>,
    right: Option<String>,
    panel: bool,
    active_transfers: String,
    panel_tab: String,
}

struct Handoff {
    cancelled: bool,
    source: String,
    payload: Option<HostWindowTransfer>,
}

#[derive(Default)]
pub(crate) struct Handoffs(Mutex<HashMap<String, Handoff>>);

impl Handoffs {
    pub(crate) fn close(&self, label: &str, state: &AppState) {
        let mut handoffs = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(transfer) = handoffs.remove(label).and_then(|h| h.payload) {
            if let Some(terminal) = transfer.terminal.and_then(|id| state.sessions.get(&id)) {
                terminal.spec.on_data.resume(None);
                terminal.spec.on_event.resume(None);
            }
        }
        handoffs.retain(|_, handoff| handoff.source != label);
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn host_window_pause(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: SessionId,
    pause: bool,
) -> ApiResult<()> {
    let session = state.sessions.get(&id).ok_or(ApiError::NotFound)?;
    let owner = session.owner.lock().unwrap_or_else(PoisonError::into_inner);
    if owner.as_str() != window.label() {
        return Err(ApiError::NotFound);
    }
    if pause {
        session.spec.on_event.pause();
        session
            .spec
            .on_data
            .pause_after(RawBytes(Vec::new()))
            .map_err(|_| ApiError::Cancelled)?;
    } else {
        session.spec.on_data.resume(None);
        session.spec.on_event.resume(None);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn host_window_take(
    window: WebviewWindow,
    state: State<'_, AppState>,
    handoffs: State<'_, Handoffs>,
    on_data: Channel<RawBytes>,
    on_event: Channel<SessionEvent>,
) -> ApiResult<Option<HostWindowTransfer>> {
    if state.vault.state()? != ibc_shell_vault::VaultState::Unlocked {
        return Err(ApiError::VaultLocked);
    }
    let mut entries = handoffs.0.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(handoff) = entries.get_mut(window.label()) else {
        if window.url().is_ok_and(|url| {
            url.query_pairs()
                .any(|(key, value)| key == "handoff" && value == "true")
        }) {
            return Err(ApiError::Cancelled);
        }
        return Ok(None);
    };
    if handoff.cancelled {
        return Err(ApiError::Cancelled);
    }
    let Some(payload) = handoff.payload.as_ref() else {
        return Ok(None);
    };
    let terminal = payload
        .terminal
        .map(|id| state.sessions.get(&id).ok_or(ApiError::NotFound))
        .transpose()?;
    let files = payload
        .files
        .map(|id| state.sftps.get(&id).ok_or(ApiError::NotFound))
        .transpose()?;
    if let Some(terminal) = terminal {
        *terminal
            .owner
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = window.label().into();
        terminal.spec.on_data.resume(Some(on_data));
        terminal.spec.on_event.resume(Some(on_event));
    }
    if let Some(files) = files {
        *files.owner.lock().unwrap_or_else(PoisonError::into_inner) = window.label().into();
    }
    Ok(handoff.payload.take())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn host_window_transferred(
    window: WebviewWindow,
    handoffs: State<'_, Handoffs>,
    label: String,
    cancel: bool,
) -> ApiResult<bool> {
    let mut entries = handoffs.0.lock().unwrap_or_else(PoisonError::into_inner);
    let transferred = entries
        .get(&label)
        .is_some_and(|h| h.source == window.label() && h.payload.is_none());
    if transferred {
        entries.remove(&label);
    } else if cancel
        && entries
            .get(&label)
            .is_some_and(|h| h.source == window.label())
    {
        // Keep the entry so a late destination sees a cancelled handoff instead of opening a new session.
        if let Some(handoff) = entries.get_mut(&label) {
            handoff.cancelled = true;
        }
    }
    Ok(transferred)
}
