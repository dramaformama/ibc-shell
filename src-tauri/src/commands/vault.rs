//! Vault commands (`ipc.md` §vault). Passwords arrive as `SecretInput`.

use std::path::Path;

use ibc_vault::{ImportSummary, VaultMode};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::api::dto::{SecretInput, VaultStatus};
use crate::api::events::{TreeChanged, VaultLocked};
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

/// Runs a blocking (Argon2 / file) vault job off the async runtime.
async fn blocking<T: Send + 'static>(
    state: &AppState,
    f: impl FnOnce(&ibc_vault::Vault) -> Result<T, ibc_vault::VaultError> + Send + 'static,
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
pub async fn vault_unlock(state: State<'_, AppState>, password: Option<SecretInput>) -> ApiResult<()> {
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

/// Writes a `.tlvault` backup (password ≥ 10 chars).
#[tauri::command]
#[specta::specta]
pub async fn vault_export(state: State<'_, AppState>, path: String, password: SecretInput) -> ApiResult<()> {
    let pw = password.0;
    blocking(&state, move |v| v.export_backup(Path::new(&path), &pw)).await
}

/// Merges a `.tlvault` backup; emits `tree://changed`.
#[tauri::command]
#[specta::specta]
pub async fn vault_import(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    password: SecretInput,
) -> ApiResult<ImportSummary> {
    let pw = password.0;
    let summary = blocking(&state, move |v| v.import_backup(Path::new(&path), &pw)).await?;
    let _ = TreeChanged.emit(&app);
    Ok(summary)
}
