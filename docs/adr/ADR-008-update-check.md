# ADR-008: Update check via GitHub Releases

Status: accepted. Changes the CSP (`connect-src` + `https://api.github.com`) and adds the setting `app.update_check`; no IPC command added (reuses `open_external`).

Decision: a UI-side read-only GET of the latest release, version compare, a link to the release page. No auto-update, no telemetry, no identifiers. Rejected: `tauri-plugin-opener` / updater plugin (more permissions than a single link needs), a Rust HTTP client (new dependencies for one GET).
Details: `docs/contracts/security.md` § Update check.
