# Security review — 2026-11-07
Scope: `src-tauri/`, `crates/ssh`, `crates/sftp`, `crates/vault`, UI IPC surface. Decisions: ADR-007.

## Findings
| # | Severity | Status | Where | Finding |
|---|---|---|---|---|
| 1 | High | fixed | `commands/vault.rs` | `vault_export` / `vault_import` took a UI path (arbitrary file write / read from a compromised webview). Now backend save/open dialog; backup file capped at 256 MiB (`backup.rs`). |
| 2 | High | fixed | `vault/idle.rs`, `lib.rs`, `src-tauri/autolock.rs` | Auto-lock was a UI timer only. Vault now enforces the idle deadline on every DEK use (zeroize + `Locked`), sweeper emits `vault://locked`, setting `security.autolock_minutes` applied at start and on `settings_set`. |
| 3 | Medium | fixed | `ssh/verify.rs`, `store/known_hosts.rs`, `connect.rs` | TOFU race: a key accepted in a slow prompt was upserted over a key stored meanwhile (silent key replacement). Now re-check after the prompt + `known_host_add_new` never replaces → hard block. |
| 4 | Low | fixed | `ssh/verify.rs` | Server auth banner kept raw (control chars, unbounded). Now plain text, ≤ 4096 chars. |
| 5 | Medium | fixed | `state.rs`, `session_task.rs` | Shell input queue unbounded; a hostile server that never opens its window stalls `shell.write` so `session_close` cannot interrupt it (leaked task/connection). Fix: byte-bounded queue + close signal raced against the write. Byte-bounded queue (1 MiB) and an out-of-band close signal are in `state.rs`. |
| 6 | Medium | fixed | `tauri.conf.json`, `lib.rs` | No navigation / new-window guard. Fix: create the window in `setup` with `on_navigation` (app origins only) and `on_new_window → Deny`. Window is created in `window.rs` with `on_navigation` (app pages only) and `on_new_window → Deny`. |
| 7 | Medium | fixed | `sftp/remote.rs`, `scan.rs` | Hostile server can return endless `readdir` batches (memory DoS in `list` / recursive scan). Fix: cap entries per listing and per scan. A listing or scan over 2,000,000 entries fails with a protocol error (`MAX_DIR_ENTRIES`). |
| 8 | Low | fixed | `sftp/scan.rs` | Names unique only case-sensitively: on case-insensitive local FS two remote files can share one `.tlpart` in parallel. Fix: case-folded `unique()` for local destinations (test `unique_folds_case_for_local_destinations`). |
| 9 | Low | fixed | `sftp/job.rs` | `ResumeIfSmaller` on a non-prefix existing file overwrites it after the 1 MiB tail check fails; resume integrity only checks the last 1 MiB. Now the first and last 1 MiB are compared before touching an existing file; a mismatch keeps it and writes the copy as `name (n).ext` (e2e `conflict_policies`). |
| 10 | Low | fixed | `state.rs` | `ShellCmd` derives `Debug` incl. keystroke bytes (typed passwords). Not logged today; make Debug redacted. `WriteBytes` has a redacted `Debug`. |
| 11 | Low | fixed | `lib.rs` | security.md's release panic hook (strip messages) is not installed. Release builds install a hook that prints only the location. |
| 12 | Low | fixed | `capabilities/default.json` | `core:event:default` also grants `emit`/`emit_to`; `allow-listen` + `allow-unlisten` suffice (drag-drop, `vault://locked`). Capability now lists `allow-listen` and `allow-unlisten`. |
| 13 | Low | fixed | `sftp/remote.rs` `map_err` | Non-status russh-sftp errors are not control-char filtered (status messages are). Control characters are filtered. |
| 14 | Info | fixed | `identity.rs` | Non-UTF-8 key file: `FromUtf8Error` buffer dropped without zeroize. `from_utf8` on the borrowed bytes, no unzeroized copy. |

Checked, no issue: secrets never in IPC responses / error strings (fixed strings, IO reduced to kind);
`AuthMethod`, `Vault`, `SecretInput`, identity inputs have redacted `Debug`; no `unwrap/expect/panic`
outside tests; changed key has no accept path (incl. jump hops, prompt timeout → reject);
remote names sanitized (traversal, reserved, NFC, length, symlinks skipped, dirs canonical-checked);
russh-sftp caps incoming packets at 256 KiB; terminal output credit window 1 MiB; CSP strict;
no `{@html}`; OSC 52 off (no clipboard addon); OSC 8 / links http(s) only after confirmation.
`cargo deny check`: ok (advisories, bans, licenses, sources). `cargo audit`: not installed.

## Residual risks
- Backend idle timer counts IPC commands only: pointer-only activity does not extend it.
- Open SSH sessions keep running after auto-lock (by design).
- Local symlinks planted between scan and write (local attacker) are followed.
- Resume compares only the first and last 1 MiB of the partial file; bytes in between are trusted.
