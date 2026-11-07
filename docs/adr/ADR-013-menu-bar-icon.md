# ADR-013: Menu-bar (tray) icon with a server summary

Status: accepted. Enables the Tauri feature `tray-icon` (core, not a plugin), adds the IPC command `tray_set` (passive), the event `tray://open-host`, the settings key `ui.tray` (bool, on by default; mirrored in localStorage `ibc.tray` so it applies before unlock), and on macOS the overlay title bar (`titleBarStyle: Overlay`, hidden title, traffic lights inside the sidebar) with the `core:window:allow-start-dragging` / `allow-internal-toggle-maximize` permissions for the toolbar drag regions.

Context: owners want to glance at their servers without bringing the window forward, like SwiftServer's menu-bar widget.

Decision: the UI builds the menu (it owns the data and the translations) and sends ready text lines: one per open or "connect at start" server with a coloured state dot and CPU / RAM (disk when ≥ 85 %), plus "Open iBC Shell" and "Quit". At most one `tray_set` per second and only when the text changed. Clicking a server line shows the window and emits `tray://open-host`; the UI opens that host's Status page. The icon is drawn in code from the logo (prompt chevron and dot) as a macOS template image. Locking the vault drops every server line on the backend side before the lock event, so no host names stay in the menu. Mobile builds compile a no-op stub.

Rejected: a second webview window as a popover (heavier, needs a window plugin and its own lifecycle); polling metrics from the backend for the tray (would duplicate the stats loop and its settings).
