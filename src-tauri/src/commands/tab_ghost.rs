//! Floating copy of a tab dragged out of its window (ADR-027).
//!
//! A webview cannot draw outside its window, so while a tab is torn off past the window edge a
//! tiny borderless, unfocusable, click-through window follows the pointer instead.

use tauri::WebviewWindow;

use crate::api::{ApiError, ApiResult};

/// Label of the single ghost window.
pub(crate) const LABEL: &str = "tab-ghost";
/// Ghost size in logical pixels.
const WIDTH: f64 = 200.0;
const HEIGHT: f64 = 34.0;
/// Pointer position inside the ghost (it is held near its left end, like the tab).
const GRAB_X: f64 = 28.0;
const GRAB_Y: f64 = HEIGHT / 2.0;

fn allowed(window: &WebviewWindow) -> ApiResult<()> {
    let label = window.label();
    if label == "main" || label.starts_with("host-") {
        Ok(())
    } else {
        Err(ApiError::Unsupported)
    }
}

/// Query string of the ghost page: name, optional flag code (`de`, `gb-sct`), theme.
fn ghost_query(title: &str, flag: Option<&str>, dark: bool) -> String {
    let title: String = title.chars().take(120).collect();
    let mut url = tauri::Url::parse("tauri://localhost/tab-ghost.html").map_or_else(
        |_| String::new(),
        |mut url| {
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("t", &title);
                if let Some(cc) = flag.filter(|cc| {
                    (2..=12).contains(&cc.len())
                        && cc.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                }) {
                    q.append_pair("f", cc);
                }
                q.append_pair("d", if dark { "1" } else { "0" });
            }
            url.query().unwrap_or_default().to_owned()
        },
    );
    url.insert_str(0, "tab-ghost.html?");
    url
}

/// Creates the hidden ghost for the tab being dragged (replaces a previous one).
#[tauri::command]
#[specta::specta]
pub(crate) async fn tab_ghost_open(
    window: WebviewWindow,
    title: String,
    flag: Option<String>,
    dark: bool,
) -> ApiResult<()> {
    allowed(&window)?;
    #[cfg(desktop)]
    {
        use tauri::Manager;
        if let Some(old) = window.app_handle().get_webview_window(LABEL) {
            let _ = old.destroy();
        }
        crate::window::create_ghost(
            window.app_handle(),
            LABEL,
            ghost_query(&title, flag.as_deref(), dark),
            WIDTH,
            HEIGHT,
        )
        .map_err(|_| ApiError::Internal("could not open tab ghost".into()))
    }
    #[cfg(not(desktop))]
    {
        let _ = (title, flag, dark);
        Err(ApiError::Unsupported)
    }
}

/// Moves the ghost to the pointer (screen coordinates, logical pixels) and shows or hides it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn tab_ghost_move(
    window: WebviewWindow,
    x: f64,
    y: f64,
    visible: bool,
) -> ApiResult<()> {
    allowed(&window)?;
    #[cfg(desktop)]
    {
        use tauri::Manager;
        let Some(ghost) = window.app_handle().get_webview_window(LABEL) else {
            return Ok(());
        };
        if !x.is_finite() || !y.is_finite() {
            return Err(ApiError::Validation("bad position".into()));
        }
        if visible {
            let _ = ghost.set_position(tauri::LogicalPosition::new(x - GRAB_X, y - GRAB_Y));
            if !ghost.is_visible().unwrap_or(false) {
                let _ = ghost.show();
            }
        } else {
            let _ = ghost.hide();
        }
        Ok(())
    }
    #[cfg(not(desktop))]
    {
        let _ = (x, y, visible);
        Err(ApiError::Unsupported)
    }
}

/// Removes the ghost (drop, cancel).
#[tauri::command]
#[specta::specta]
pub(crate) async fn tab_ghost_close(window: WebviewWindow) -> ApiResult<()> {
    allowed(&window)?;
    #[cfg(desktop)]
    {
        use tauri::Manager;
        if let Some(ghost) = window.app_handle().get_webview_window(LABEL) {
            let _ = ghost.destroy();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encodes_title_and_drops_bad_flags() {
        let q = ghost_query("web & db #1", Some("de"), true);
        assert_eq!(q, "tab-ghost.html?t=web+%26+db+%231&f=de&d=1");
        let q = ghost_query("x", Some("../evil"), false);
        assert_eq!(q, "tab-ghost.html?t=x&d=0");
        let long = "я".repeat(500);
        assert!(ghost_query(&long, None, false).len() < 1200);
    }
}
