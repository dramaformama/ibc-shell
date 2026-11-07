//! The main window, created in code so it can be locked to the app's own pages.
//!
//! A compromised page (or a link in remote data that slipped through) must not be able to
//! navigate the webview to a foreign site, where the IPC bridge would then be reachable, nor
//! open extra windows. Foreign links go through the UI's confirmation and the system browser.

use tauri::webview::WebviewWindowBuilder;
use tauri::Url;

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
    let Some(cfg) = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
    else {
        return Err("main window is not configured".into());
    };
    let builder = WebviewWindowBuilder::from_config(app.handle(), &cfg)?
        .on_navigation(is_app_page);
    #[cfg(desktop)]
    let builder = builder.on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
    builder.build()?;
    Ok(())
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
