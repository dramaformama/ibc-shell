# iBC Shell — project rules for Claude Code

Cross-platform SSH + SFTP client (macOS, Windows, Linux, iOS, Android).
Goals: lightweight, minimal, beautiful, fast, modern, secure. Server folders + country flags.
Name: "iBC Shell" (crates `ibc-*`, identifier `app.ibcserver`).

- Full plan & phases: `docs/PLAN.md` (Russian, for humans + orchestrator).
- Source of truth for code: `docs/contracts/*.md` (FROZEN after Phase 0; changes only via ADR in `docs/adr/`).
- Current progress: `docs/STATUS.md` (orchestrator keeps it ≤ 40 lines).
- Read ONLY the contract sections relevant to your task.

## Stack (frozen)
- Shell: Tauri 2.x (desktop + mobile targets). Plugins allowed: dialog, os, updater, window-state, single-instance, biometric (mobile), clipboard-manager. Nothing else without ADR.
- Core: Rust stable (pinned in `rust-toolchain.toml`), tokio, thiserror, tracing.
- SSH: `russh` (+ its `keys` module), SFTP: `russh-sftp`.
- Storage: SQLite via `rusqlite` (feature `bundled`) + `rusqlite_migration`. WAL mode.
- Crypto: `chacha20poly1305` (XChaCha20-Poly1305), `argon2` (Argon2id), `zeroize`, `secrecy`, `rand` (OsRng only), `uuid` v7.
- OS secret storage: `keyring` crate (macOS/iOS Keychain, Windows Credential Manager, Linux Secret Service). Android: own Tauri plugin over Android Keystore.
- IPC: `tauri-specta` → generated TS bindings in `ui/src/lib/api/bindings.ts`. Streams via `tauri::ipc::Channel` with RAW bytes (never JSON number arrays).
- UI: Svelte 5 (runes) + Vite + TypeScript strict + Tailwind CSS v4 + bits-ui (headless) + lucide-svelte. i18n: paraglide-js (ru, en).
- Terminal: `@xterm/xterm` 5.x + addons: webgl, fit, search, web-links, unicode11, clipboard (OSC52 off by default).
- Flags: circle-flags SVGs (MIT) served as static files `ui/static/flags/{cc}.svg`; country names via `Intl.DisplayNames` (no data bundle).
- Package manager: pnpm. Task runner: `just`.

## Repo layout
```
crates/store   # SQLite, models, migrations, folder tree + inheritance
crates/vault   # DEK/KEK, secret encryption, keychain, backup format
crates/ssh     # russh client: connect, auth, host keys, PTY, forwards, jump
crates/sftp    # SFTP ops + transfer engine (queue, pipelining, resume)
crates/geo     # optional offline IP→country (feature-gated, off by default)
src-tauri      # app shell: commands, channels, state, capabilities, mobile plugins
ui             # Svelte app
tests/docker   # OpenSSH test servers (compose)
docs           # PLAN, contracts, adr, STATUS
```
Dependency rule: `ssh` and `sftp` must NOT depend on `store`/`vault`. Only `src-tauri` wires crates together.

## Commands
- Dev: `pnpm tauri dev` · Mobile: `pnpm tauri android dev` / `pnpm tauri ios dev`
- Rust: `cargo test -p <crate>` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo deny check`
- UI: `pnpm -C ui check` · `pnpm -C ui test` · `pnpm -C ui lint`
- Integration: `just e2e` (starts `tests/docker/compose.yml`)
- Bindings: `just bindings` (regenerates `ui/src/lib/api/bindings.ts`)

## Hard rules
1. Secrets (passwords, private keys, passphrases, DEK/KEK) never cross into JS except the single unavoidable user input (typed password/passphrase), which is wrapped in `SecretString` on arrival and never echoed back. Frontend receives IDs and fingerprints only.
2. No secrets in logs, errors, panics, Debug impls, IPC responses, crash reports. Implement `Debug` manually for any struct holding secrets.
3. No `unwrap()`/`expect()`/`panic!` in non-test code under `crates/` and `src-tauri/`.
4. Host key mismatch = hard block. Never auto-accept a changed key.
5. Every IPC command/event exists in `docs/contracts/ipc.md` before implementation.
6. Never render server-provided strings as HTML (`{@html}` forbidden on remote data). Sanitize remote file names before touching the local FS.
7. New dependency → one-line justification in your report; prefer existing deps. `cargo deny` must pass (licenses: MIT/Apache-2.0/BSD/ISC/Zlib/MPL-2.0/Unicode).
8. Files ≤ 400 lines; one module = one responsibility. Public items documented.
9. UI: no heavy component libraries, no CSS-in-JS runtime, no moment/lodash. Virtualize any list that can exceed 200 rows.

## Token economy (all agents)
- Work only inside the paths your agent definition owns. Read other code only via its public API file listed in your definition.
- Grep/Glob before Read; Read with offset/limit; never read lockfiles, generated bindings or `node_modules`/`target`.
- Run only your crate's/package's tests, not the whole workspace (orchestrator runs gates).
- Final report ≤ 150 words: done / files touched / tests status / open issues / contract change requests. No code dumps, no diffs.
- If blocked by a contract gap: stop, report the exact question. Do not invent APIs in other areas.
