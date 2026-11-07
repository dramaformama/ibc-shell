//! Mobile builds have no menu bar: the tray command accepts the menu and does nothing (ADR-013).

use ibc_shell_store::HostId;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// One server line (same shape as on desktop).
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct TrayItem {
    /// Host to open on click.
    pub host_id: Option<HostId>,
    /// Text as shown.
    pub text: String,
}

/// The whole menu (same shape as on desktop).
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct TrayMenu {
    /// Hover text.
    pub tooltip: String,
    /// Server lines.
    pub items: Vec<TrayItem>,
    /// "Open iBC Shell".
    pub open_label: String,
    /// "Quit".
    pub quit_label: String,
}

/// No-op on mobile.
#[allow(clippy::unnecessary_wraps)]
pub fn apply(_app: &AppHandle, _menu: Option<TrayMenu>) -> tauri::Result<()> {
    Ok(())
}

/// No-op on mobile.
pub fn on_lock(_app: &AppHandle) {}
