//! Vault commands (`ipc.md` §vault). Passwords arrive as `SecretInput`.

use std::path::PathBuf;

use ibc_shell_vault::{ImportSummary, VaultMode, VaultState};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_specta::Event;

use crate::api::dto::{SecretInput, VaultStatus};
use crate::api::events::{TreeChanged, VaultLocked};
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

/// Runs a blocking (Argon2 / file) vault job off the async runtime.
async fn blocking<T: Send + 'static>(
    state: &AppState,
    f: impl FnOnce(&ibc_shell_vault::Vault) -> Result<T, ibc_shell_vault::VaultError> + Send + 'static,
) -> ApiResult<T> {
    let vault = state.vault.clone();
    tauri::async_runtime::spawn_blocking(move || f(&vault))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(ApiError::from)
}

/// Lock state, mode, biometric availability.
#[tauri::command]
#[specta::specta]
pub async fn vault_status(state: State<'_, AppState>) -> ApiResult<VaultStatus> {
    Ok(VaultStatus {
        state: state.vault.state()?,
        mode: state.vault.mode()?,
        biometric_available: state.vault.biometric_available(),
    })
}

/// Initializes the vault (`MasterPassword` requires `password`); leaves it unlocked.
#[tauri::command]
#[specta::specta]
pub async fn vault_init(
    state: State<'_, AppState>,
    mode: VaultMode,
    password: Option<SecretInput>,
) -> ApiResult<()> {
    let pw = password.map(|p| p.0);
    blocking(&state, move |v| v.init(mode, pw.as_ref())).await
}

/// Unlocks; `password = null` → keychain / biometric.
#[tauri::command]
#[specta::specta]
pub async fn vault_unlock(
    state: State<'_, AppState>,
    password: Option<SecretInput>,
) -> ApiResult<()> {
    let pw = password.map(|p| p.0);
    blocking(&state, move |v| match &pw {
        Some(p) => v.unlock_with_password(p),
        None => v.unlock_with_keystore(),
    })
    .await
}

/// Locks (zeroizes DEK) and emits `vault://locked`. Open sessions keep running.
#[tauri::command]
#[specta::specta]
pub async fn vault_lock(app: AppHandle, state: State<'_, AppState>) -> ApiResult<()> {
    state.vault.lock();
    crate::tray::on_lock(&app);
    let _ = VaultLocked.emit(&app);
    Ok(())
}

/// Rewraps the DEK.
#[tauri::command]
#[specta::specta]
pub async fn vault_change_password(
    state: State<'_, AppState>,
    old_password: SecretInput,
    new_password: SecretInput,
) -> ApiResult<()> {
    let (o, n) = (old_password.0, new_password.0);
    blocking(&state, move |v| v.change_password(&o, &n)).await
}

/// Backup file extension (without the dot).
const BACKUP_EXT: &str = "tlvault";

pub(crate) fn require_unlocked(state: &AppState) -> ApiResult<()> {
    match state.vault.state()? {
        VaultState::Unlocked => Ok(()),
        _ => Err(ApiError::VaultLocked),
    }
}

fn picked_path(p: FilePath) -> ApiResult<PathBuf> {
    p.into_path()
        .map_err(|_| ApiError::Validation("unsupported file location".into()))
}

/// Shows the native "save" dialog (backend side, ADR-007) and writes a `.tlvault` backup
/// there (password ≥ 10 chars). The webview never names the path. `false` = cancelled.
#[tauri::command]
#[specta::specta]
pub async fn vault_export(
    app: AppHandle,
    state: State<'_, AppState>,
    password: SecretInput,
) -> ApiResult<bool> {
    require_unlocked(&state)?;
    let pw = password.0;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("iBC Shell backup", &[BACKUP_EXT])
            .set_file_name(format!("ibc-shell-backup.{BACKUP_EXT}"))
            .blocking_save_file()
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    let Some(picked) = picked else {
        return Ok(false);
    };
    let path = picked_path(picked)?;
    blocking(&state, move |v| v.export_backup(&path, &pw)).await?;
    Ok(true)
}

/// Shows the native "open" dialog (backend side, ADR-007) and merges the chosen `.tlvault`
/// backup; emits `tree://changed`. `null` = cancelled.
#[tauri::command]
#[specta::specta]
pub async fn vault_import(
    app: AppHandle,
    state: State<'_, AppState>,
    password: SecretInput,
) -> ApiResult<Option<ImportSummary>> {
    require_unlocked(&state)?;
    let pw = password.0;
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("iBC Shell backup", &[BACKUP_EXT])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked_path(picked)?;
    let summary = blocking(&state, move |v| v.import_backup(&path, &pw)).await?;
    let _ = TreeChanged.emit(&app);
    Ok(Some(summary))
}
