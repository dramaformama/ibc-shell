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

/// Notification text: no control characters, at most `max` characters.
fn notice_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// Shows a system notification (a command in a background terminal finished, ADR-017).
/// The backend shows it, so the webview needs no notification permission.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)] // Tauri injects the handle by value.
pub fn app_notify(app: tauri::AppHandle, title: String, body: String) -> ApiResult<()> {
    #[cfg(desktop)]
    {
        use tauri_plugin_notification::NotificationExt;
        app.notification()
            .builder()
            .title(notice_text(&title, 120))
            .body(notice_text(&body, 300))
            .show()
            .map_err(|_| ApiError::Unsupported)
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, title, body);
        Err(ApiError::Unsupported)
    }
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
    use super::{is_web_url, notice_text};

    #[test]
    fn notice_text_is_plain_and_short() {
        assert_eq!(notice_text("a\u{1b}[31mb\nc", 10), "a[31mbc");
        assert_eq!(notice_text("абвгд", 3), "абв");
    }

    #[test]
    fn url_validation() {
        assert!(is_web_url("https://example.com/a?b=1"));
        assert!(!is_web_url("file:///etc/passwd"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("https://a b"));
    }
}
