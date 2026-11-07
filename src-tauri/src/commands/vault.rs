//! Vault commands (`ipc.md` §vault). Passwords arrive as `SecretInput`.

use termlite_vault::{ImportSummary, VaultMode};

use crate::api::dto::{SecretInput, VaultStatus};
use crate::api::{ApiError, ApiResult};

/// Lock state, mode, biometric availability.
#[tauri::command]
#[specta::specta]
pub async fn vault_status() -> ApiResult<VaultStatus> {
    Err(ApiError::unimplemented())
}

/// Initializes the vault (`MasterPassword` requires `password`); leaves it unlocked.
#[tauri::command]
#[specta::specta]
pub async fn vault_init(mode: VaultMode, password: Option<SecretInput>) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Unlocks; `password = null` → keychain / biometric.
#[tauri::command]
#[specta::specta]
pub async fn vault_unlock(password: Option<SecretInput>) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Locks (zeroizes DEK) and emits `vault://locked`.
#[tauri::command]
#[specta::specta]
pub async fn vault_lock() -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Rewraps the DEK.
#[tauri::command]
#[specta::specta]
pub async fn vault_change_password(
    old_password: SecretInput,
    new_password: SecretInput,
) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Writes a `.tlvault` backup (password ≥ 10 chars).
#[tauri::command]
#[specta::specta]
pub async fn vault_export(path: String, password: SecretInput) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}

/// Merges a `.tlvault` backup; emits `tree://changed`.
#[tauri::command]
#[specta::specta]
pub async fn vault_import(path: String, password: SecretInput) -> ApiResult<ImportSummary> {
    Err(ApiError::unimplemented())
}
