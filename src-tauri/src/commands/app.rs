//! App commands (`ipc.md` §app).

use crate::api::dto::{AppInfo, TrayMenu};
use crate::api::{ApiError, ApiResult};

/// Version / platform / arch.
#[tauri::command]
#[specta::specta]
pub async fn app_info() -> ApiResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        platform: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
    })
}

/// Shows or updates the menu-bar icon with the given lines; `null` hides it (ADR-013).
/// Sync on purpose: runs on the main thread, where native menus live.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)] // Tauri injects the handle by value.
pub fn tray_set(app: tauri::AppHandle, menu: Option<TrayMenu>) -> ApiResult<()> {
    crate::tray::apply(&app, menu).map_err(|_| ApiError::Internal("tray unavailable".into()))
}

/// True for a plain http/https URL without whitespace or control characters.
fn is_web_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && url.len() > 8
        && url.len() <= 2048
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Opens an http/https URL in the system browser (validated).
#[tauri::command]
#[specta::specta]
pub async fn open_external(url: String) -> ApiResult<()> {
    if !is_web_url(&url) {
        return Err(ApiError::Validation(
            "only http/https URLs are allowed".into(),
        ));
    }
    let (prog, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![&url])
    } else if cfg!(windows) {
        ("rundll32", vec!["url.dll,FileProtocolHandler", &url])
    } else {
        ("xdg-open", vec![&url])
    };
    std::process::Command::new(prog)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|_| ApiError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::is_web_url;

    #[test]
    fn url_validation() {
        assert!(is_web_url("https://example.com/a?b=1"));
        assert!(!is_web_url("file:///etc/passwd"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("https://a b"));
    }
}
