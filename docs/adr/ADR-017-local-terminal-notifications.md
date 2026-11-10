# ADR-017: Local terminal, background-command notifications, SQLite update

Status: accepted (version 1.0.0). Adds the IPC commands `session_open_local` and `app_notify`, the plugin `tauri-plugin-notification`, the dependency `portable-pty` (desktop only), and replaces `rusqlite_migration`.

## Local terminal
Context: owners keep a system terminal open next to the app only to run local commands (`ssh-keygen`, `scp`, git). A terminal inside the app, always running, saves the round trip.

Decision:
- `SessionTarget::Local`: the same session registry, channels, coalescing (64 KiB / 8 ms) and credit window (1 MiB) as SSH (ADR-004). `shell_write` / `shell_ack` / `shell_resize` / `session_close` work unchanged; `session_reconnect` starts a new shell.
- `portable-pty` (MIT, the PTY layer of WezTerm) opens a login shell (`$SHELL`, or the Windows default) in the home folder with `TERM=xterm-256color`, `COLORTERM=truecolor`, and `LANG=en_US.UTF-8` only when the app was started without a locale. A reader thread feeds a bounded channel; keystrokes go to a writer thread and stay counted against the input budget until written.
- Desktop only (`cfg(desktop)`); on mobile the session fails with `Validation`.
- UI: the local terminal is a tab with `local: true`, started in the background when the app opens, shown from its own sidebar row ("Local terminal", ⌃\`), never in the tab bar, never closed. When the shell exits it is restarted.

Security: the local shell runs as the user, which the user can already do; the webview gets no new capability (the command spawns only the user's own login shell, no program or arguments come from JS).

## Notifications
Context: a long command in a background tab (or while the user is in another app) should say when it finished.

Decision: the UI watches the output of terminals the user is not looking at (`terminal/activity.ts`): a bell outside OSC strings, or a shell prompt at the end of the output after the tab has been hidden for ≥ 5 s, gives one notice per hidden stint. In the app it is a toast; with the app in the background, `app_notify(title, body)` shows a system notification from the backend (`tauri-plugin-notification`), so the webview has no notification permission. Text is stripped of control characters and capped (120 / 300). It is a heuristic over the output: the server's shell is not instrumented. Settings → Terminal can switch it off.

`cargo deny`: the plugin pulls `zbus` on Linux; three "unmaintained" notices (RUSTSEC-2026-0370/0371/0372, no vulnerability) are ignored with that reason.

## Terminal ↔ Files
The UI follows the shell's folder from OSC 7 (`file://host/path`) or, failing that, the Debian / Ubuntu title `user@host: dir` (`terminal/cwd.ts`, absolute or `~` paths only, control characters rejected). "Open this folder in Files" switches the server to Files at that folder. Files dropped from the OS onto a server terminal are uploaded there over an SFTP channel of the terminal's own connection with the `Rename` conflict policy (there is no conflict dialog outside Files); onto the local terminal they are typed in as shell-quoted paths. No new IPC.

## SQLite
RUSTSEC-2026-0459 (CVE in the bundled SQLite) is fixed in `libsqlite3-sys` 0.39 / `rusqlite` 0.41, which `rusqlite_migration` 2.6 does not accept. The single migration now runs through a 20-line migrator in `crates/store/src/schema.rs` with the same `PRAGMA user_version` scheme, so existing databases open unchanged; a database from a newer app is refused instead of being touched.

Rejected: HTML5 drag-and-drop for tab reordering (unreliable in the webview; pointer events are used); notification permission in JS (needs a capability); shell integration scripts injected into remote shells (changes the server).
