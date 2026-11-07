# ADR-001: Tauri 2 as the application shell
Date: 2026-11-07 · Status: Accepted

## Context
One codebase for macOS, Windows, Linux, iOS, Android; small installers (≤ 15 MB), low idle RAM,
Rust core for SSH/crypto. Electron (Termius) ships 100+ MB and ~300 MB RAM; Flutter's SSH/terminal
stack (dartssh2, xterm.dart) is less mature than russh + xterm.js.

## Decision
Tauri 2.x (pinned via Cargo.lock; 2.12.5 at freeze) with the system WebView, SvelteKit static
adapter (SPA, `ssr = false`) in `ui/`. Plugins limited to the CLAUDE.md list. IPC types via
`tauri-specta =2.0.0-rc.26` + `specta =2.0.0-rc.26` + `specta-typescript =0.0.13` (exact pins: pre-1.0,
tauri-specta requires matching specta). Bindings exported by `cargo run -p ibc-server --example export_bindings`
(`just bindings`), unified serde mode (`disable_serde_phases`), `ErrorHandlingMode::Result`.

## Consequences
WebView differences per OS (WebKitGTK on Linux, WebView2 on Windows) must be tested; mobile
targets are less mature (PLAN §8). Specta RCs may break on upgrade → bump only via ADR together
with regenerated bindings. `src-tauri` test `bindings_are_fresh` guards drift. Contracts: ipc.md v1.
