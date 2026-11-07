//! App commands (`ipc.md` §app).

use crate::api::dto::AppInfo;
use crate::api::{ApiError, ApiResult};

/// Version / platform / arch.
#[tauri::command]
#[specta::specta]
pub async fn app_info() -> ApiResult<AppInfo> {
    Err(ApiError::unimplemented())
}

/// Opens an http/https URL in the system browser (validated).
#[tauri::command]
#[specta::specta]
pub async fn open_external(url: String) -> ApiResult<()> {
    Err(ApiError::unimplemented())
}
