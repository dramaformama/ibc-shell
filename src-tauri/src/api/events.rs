//! Global events (`app.emit`), typed via tauri-specta.

use serde::{Deserialize, Serialize};

/// Vault auto-locked (idle / sleep / background). UI shows the lock screen.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "vault://locked")]
pub struct VaultLocked;

/// Tree changed outside UI actions (after import). UI reloads `tree_list`.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "tree://changed")]
pub struct TreeChanged;

/// A server line of the menu-bar icon was clicked; the UI opens that host's Status page.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "tray://open-host")]
pub struct TrayOpenHost {
    /// Host to open.
    pub host_id: ibc_shell_store::HostId,
}

/// Signed update available.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "update://available")]
pub struct UpdateAvailable {
    /// New version.
    pub version: String,
    /// Release notes (plain text).
    pub notes: Option<String>,
}

/// A host window handed its sessions back; the main window adopts them into a tab
/// (`host_window_adopt`). Sent to the main window only (ADR-027).
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "host-window://return")]
pub struct HostWindowReturn {
    /// Handoff label for `host_window_adopt`.
    pub label: String,
    /// Host the sessions belong to.
    pub host_id: ibc_shell_store::HostId,
    /// View the tab was showing.
    pub view: crate::commands::host_window::HostWindowView,
}
