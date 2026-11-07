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

/// Signed update available.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "update://available")]
pub struct UpdateAvailable {
    /// New version.
    pub version: String,
    /// Release notes (plain text).
    pub notes: Option<String>,
}
