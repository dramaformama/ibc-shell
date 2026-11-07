//! Menu-bar (tray) icon with a mini summary of servers (`ipc.md` §App, ADR-013). The UI sends
//! ready-made lines; the backend only draws them. Locking the vault drops the server lines.

use std::sync::Mutex;

use ibc_shell_store::HostId;
use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::api::events::TrayOpenHost;

const TRAY_ID: &str = "main";
const MAX_ITEMS: usize = 24;
const MAX_TEXT: usize = 120;
/// Icon size in pixels (18 pt at 2×).
const SIZE: u32 = 36;

/// One server line.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct TrayItem {
    /// Host to open on click; `null` = a disabled caption.
    pub host_id: Option<HostId>,
    /// Text as shown.
    pub text: String,
}

/// The whole menu, already translated by the UI.
#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct TrayMenu {
    /// Hover text.
    pub tooltip: String,
    /// Server lines (≤ 24).
    pub items: Vec<TrayItem>,
    /// "Open iBC Shell".
    pub open_label: String,
    /// "Quit".
    pub quit_label: String,
}

/// Last labels, so a lock can rebuild the menu without server lines.
static LABELS: Mutex<Option<(String, String)>> = Mutex::new(None);

fn clip(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEXT)
        .collect()
}

/// Shows (or updates) the tray icon; `None` removes it.
pub fn apply(app: &AppHandle, menu: Option<TrayMenu>) -> tauri::Result<()> {
    let Some(menu) = menu else {
        let _ = app.remove_tray_by_id(TRAY_ID);
        return Ok(());
    };
    if let Ok(mut l) = LABELS.lock() {
        *l = Some((menu.open_label.clone(), menu.quit_label.clone()));
    }
    let built = build_menu(app, &menu)?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(built))?;
        tray.set_tooltip(Some(clip(&menu.tooltip)))?;
        return Ok(());
    }
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::new_owned(glyph(), SIZE, SIZE))
        .icon_as_template(true)
        .tooltip(clip(&menu.tooltip))
        .menu(&built)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, e| on_click(app, &e))
        .build(app)?;
    Ok(())
}

/// After a lock: keep the icon, drop every server line.
pub fn on_lock(app: &AppHandle) {
    if app.tray_by_id(TRAY_ID).is_none() {
        return;
    }
    let labels = LABELS.lock().ok().and_then(|l| l.clone());
    let (open_label, quit_label) = labels.unwrap_or_else(|| ("iBC Shell".into(), "Quit".into()));
    let _ = apply(
        app,
        Some(TrayMenu {
            tooltip: "iBC Shell".into(),
            items: vec![],
            open_label,
            quit_label,
        }),
    );
}

fn build_menu(app: &AppHandle, menu: &TrayMenu) -> tauri::Result<Menu<tauri::Wry>> {
    let out = Menu::new(app)?;
    for item in menu.items.iter().take(MAX_ITEMS) {
        let (id, enabled) = match item.host_id {
            Some(h) => (format!("host:{}", h.0), true),
            None => (format!("cap:{}", out.items()?.len()), false),
        };
        out.append(&MenuItem::with_id(
            app,
            id,
            clip(&item.text),
            enabled,
            None::<&str>,
        )?)?;
    }
    if !menu.items.is_empty() {
        out.append(&PredefinedMenuItem::separator(app)?)?;
    }
    out.append(&MenuItem::with_id(
        app,
        "open",
        clip(&menu.open_label),
        true,
        None::<&str>,
    )?)?;
    out.append(&MenuItem::with_id(
        app,
        "quit",
        clip(&menu.quit_label),
        true,
        None::<&str>,
    )?)?;
    Ok(out)
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn on_click(app: &AppHandle, event: &tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    if id == "quit" {
        app.exit(0);
    } else if id == "open" {
        show_main(app);
    } else if let Some(raw) = id.strip_prefix("host:") {
        if let Ok(uuid) = uuid::Uuid::parse_str(raw) {
            show_main(app);
            let _ = TrayOpenHost {
                host_id: HostId(uuid),
            }
            .emit(app);
        }
    }
}

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

/// The logo's prompt chevron and dot as a black template image (alpha only), antialiased.
fn glyph() -> Vec<u8> {
    // Logo coordinates (100 × 100 viewBox) mapped onto the icon, content centred.
    let scale = 0.6_f32;
    let (cx, cy) = (51.0_f32, 52.0_f32);
    let half = 18.0_f32;
    let map = |x: f32, y: f32| ((x - cx) * scale + half, (y - cy) * scale + half);
    let (a, b, c) = (map(24.0, 33.0), map(46.0, 50.0), map(24.0, 67.0));
    let dot = map(67.0, 66.0);
    let stroke = 7.0 * scale / 2.0;
    let radius = 12.0 * scale;
    let mut px = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            #[allow(clippy::cast_precision_loss)]
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let d_line = seg_dist(p, a, b).min(seg_dist(p, b, c)) - stroke;
            let d_dot = ((p.0 - dot.0).powi(2) + (p.1 - dot.1).powi(2)).sqrt() - radius;
            let cover = (0.5 - d_line.min(d_dot)).clamp(0.0, 1.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let alpha = (cover * 255.0).round() as u8;
            px.extend_from_slice(&[0, 0, 0, alpha]);
        }
    }
    px
}

#[cfg(test)]
mod tests {
    #[test]
    fn glyph_has_ink_and_transparent_corners() {
        let px = super::glyph();
        assert_eq!(px.len(), (super::SIZE * super::SIZE * 4) as usize);
        assert_eq!(px[3], 0);
        assert!(px.chunks(4).filter(|p| p[3] == 255).count() > 60);
    }
}
