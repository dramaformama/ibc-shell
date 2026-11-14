//! Main and host windows, restricted to the app's own pages.
//!
//! A compromised page (or a link in remote data that slipped through) must not be able to
//! navigate the webview to a foreign site, where the IPC bridge would then be reachable, nor
//! open extra windows. Foreign links go through the UI's confirmation and the system browser.

use tauri::utils::config::WindowConfig;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, Url};

/// True for the pages the bundled UI is served from (and the dev server in `tauri dev`).
fn is_app_page(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => {
            let host = url.host_str();
            host == Some("tauri.localhost") || (cfg!(dev) && host == Some("localhost"))
        }
        _ => false,
    }
}

/// Builds the window described in `tauri.conf.json` (`"create": false`) with the guards.
pub fn create_main(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = main_config(app.handle()).ok_or("main window is not configured")?;
    build(app.handle(), &cfg)?;
    Ok(())
}

fn main_config(app: &AppHandle) -> Option<WindowConfig> {
    app.config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
}

#[cfg(desktop)]
pub(crate) fn create_host(
    app: &AppHandle,
    label: String,
    path: String,
    title: String,
) -> crate::api::ApiResult<()> {
    let mut cfg = main_config(app)
        .ok_or_else(|| crate::api::ApiError::Internal("main window is not configured".into()))?;
    cfg.label = label;
    cfg.title = title;
    cfg.url = tauri::WebviewUrl::App(path.into());
    build(app, &cfg)
        .map_err(|_| crate::api::ApiError::Internal("could not open host window".into()))
}

fn build(app: &AppHandle, cfg: &WindowConfig) -> tauri::Result<()> {
    let builder = WebviewWindowBuilder::from_config(app, cfg)?.on_navigation(is_app_page);
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let builder = builder.decorations(false);
    #[cfg(desktop)]
    let builder = builder.on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
    builder.build()?;
    Ok(())
}

pub(crate) fn cleanup(window: &tauri::Window) {
    let state = window.state::<crate::state::AppState>();
    let label = window.label();
    window
        .state::<crate::commands::host_window::Handoffs>()
        .close(label, &state);
    state.transfers.remove_subscriber(label);
    for (id, handle) in state.sessions.take_where(|h| {
        h.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_str()
            == label
    }) {
        state.pending.cancel_owner(id.0);
        handle.send(crate::state::ShellCmd::Close);
    }
    for (id, handle) in state.sftps.take_where(|h| {
        h.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_str()
            == label
    }) {
        state.pending.cancel_owner(id.0);
        tauri::async_runtime::spawn(async move { handle.sftp.close().await });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> bool {
        is_app_page(&Url::parse(s).unwrap())
    }

    #[test]
    fn only_own_pages() {
        assert!(ok("tauri://localhost/index.html"));
        assert!(ok("http://tauri.localhost/"));
        assert!(!ok("https://example.com/"));
        assert!(!ok("http://tauri.localhost.evil.com/"));
        assert!(!ok("file:///etc/passwd"));
        assert!(!ok("javascript:alert(1)"));
        assert!(!ok("data:text/html,x"));
    }
}
