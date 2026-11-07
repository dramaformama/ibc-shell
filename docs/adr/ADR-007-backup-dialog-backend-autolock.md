# ADR-007: Backup file dialogs in the backend, backend-enforced auto-lock, TOFU without overwrite
Date: 2026-11-07 · Status: Accepted (security review, `docs/security-review.md`)

## Context
ADR-006 left `vault_export` / `vault_import` taking a UI path: a compromised webview could write
a backup anywhere or read any file. Auto-lock was a UI timer only (throttled / hung webview →
DEK stays in memory). A host key accepted in one prompt could overwrite a key stored meanwhile.

## Decision
- `vault_export(password) -> boolean` and `vault_import(password) -> ImportSummary | null`: the
  backend opens the native save / open dialog (`tauri-plugin-dialog`, Rust API only, filter
  `.tlvault`); `false` / `null` = cancelled. Backup files over 256 MiB are refused before reading.
- Vault API additions: `set_idle_timeout(Option<Duration>)`, `touch()`, `lock_if_idle() -> bool`.
  Every DEK use checks the deadline; expiry zeroizes the DEK and returns `Locked`.
  The bridge applies `security.autolock_minutes` (same parsing as the UI; default 15, 0 = never)
  at start-up and on `settings_set`, runs a 5 s sweeper that emits `vault://locked`, and counts
  every invoke except `shell_ack`, `vault_status`, `transfers_subscribe` as user activity.
  The UI timer stays (instant lock on pointer idleness).
- Store: `known_host_add_new` (insert only while the host has no other key). The connect path
  uses it and re-checks the store after the TOFU prompt; a key stored meanwhile = hard block.

## Consequences
ipc.md vault rows changed, bindings regenerated. No new dependencies.
Pointer-only activity (reading output without typing) does not extend the backend deadline.
