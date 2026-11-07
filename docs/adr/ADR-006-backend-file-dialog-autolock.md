# ADR-006: Backend-side key file picker, auto-lock setting
Date: 2026-11-07 · Status: Accepted

## Context
`identity_import_file(path)` let a compromised webview make the backend read any local file.
`security.md` requires an idle auto-lock setting but gives no key name.

## Decision
`identity_import_file(passphrase: string | null) -> IdentityDto | null`: the backend opens the native
picker itself (`tauri-plugin-dialog`, Rust API only, no JS permission) and imports the chosen file
(max 64 KiB); `null` = cancelled. Setting key `security.autolock_minutes` (number, default 15,
0 = never) via `settings_get/set`; the UI idle timer calls `vault_lock`.
Capabilities: `core:event:default` only. CSP as in `security.md`, plus `font-src 'self'`, `form-action 'none'`.
paraglide's message-format plugin is read from `node_modules` (no CDN at compile time).

## Consequences
ipc.md `identity_import_file` row changed. New dependency: `tauri-plugin-dialog` (allowed plugin).
`vault_export` / `vault_import` still take a UI path: follow-up.
