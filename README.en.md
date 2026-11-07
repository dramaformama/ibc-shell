<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/logo/logo-white.svg">
    <img src="docs/brand/logo/logo-dark.svg" alt="iBC Shell" height="72">
  </picture>
</p>

# iBC Shell

**English** · [Русский](README.md)

A light SSH and SFTP client for people who run many servers.
Nested folders, country flags, a terminal that stays out of your way, and secrets that never leave your machine.

> **Status: early development.** The core (SSH, SFTP engine, encrypted vault, terminal and file UI) is built and tested.
> The pieces are not yet wired into a runnable release, so there are no installers yet. See [Download](#download).

## Why

- **Light.** Small installer, low memory. Tauri and Rust, no Electron.
- **Servers by place.** Nested folders, each host with a country flag. Find a server by where it is, not by scrolling.
- **Honest security.** Host-key changes are a hard block, never auto-accepted. The vault state is always visible.
  No accounts, no cloud, nothing phones home.
- **Keyboard first.** Command palette on <kbd>⌘K</kbd> / <kbd>Ctrl K</kbd>. Russian and English UI.
- **Quiet design.** Content first, the terminal is the hero. Light and dark themes.

## Download

All builds will be published on the [Releases page](https://github.com/dramaformama/ibc-shell/releases/latest).

| Platform                     | Download                                                               | Status            |
| ---------------------------- | ---------------------------------------------------------------------- | ----------------- |
| macOS (Apple Silicon, Intel) | [Releases](https://github.com/dramaformama/ibc-shell/releases/latest) | not built yet     |
| Windows 10/11                | [Releases](https://github.com/dramaformama/ibc-shell/releases/latest) | not built yet     |
| Linux (AppImage, deb)        | [Releases](https://github.com/dramaformama/ibc-shell/releases/latest) | not built yet     |
| iOS                          | —                                                                      | planned (Phase 4) |
| Android                      | —                                                                      | planned (Phase 4) |

Until installers exist you can [build from source](#build-from-source).

## What works today

| Area                                                                              | State                                      |
| --------------------------------------------------------------------------------- | ------------------------------------------ |
| Encrypted vault (Argon2id + XChaCha20-Poly1305), host store, folder tree          | done, tested                               |
| SSH client: password, key, keyboard-interactive, host-key checks, jump hosts      | done, tested against real OpenSSH          |
| SFTP engine: queue, resume, pause, conflict policies, remote-to-remote            | done, tested                               |
| Terminal UI (xterm.js tabs, search, paste safety)                                 | done, not yet run against the real backend |
| SFTP UI (two panes, transfer queue)                                               | done, not yet run against the real backend |
| App shell wiring (Tauri commands)                                                 | in progress                                |
| Port forwarding, `ssh_config` import, key manager, split panes, snippets, backups | planned (Phase 3)                          |
| Mobile                                                                            | planned (Phase 4)                          |

Nothing here is benchmarked against other clients yet. Numbers will go in `docs/bench.md` once they are measured.

## Build from source

You need Rust (stable), Node with pnpm, and [just](https://github.com/casey/just).

```sh
pnpm install
pnpm tauri dev          # run the app
cargo test --workspace  # Rust tests
pnpm -C ui check && pnpm -C ui test
just e2e                # integration tests against OpenSSH in Docker
```

## Layout

```
crates/store   SQLite, folder tree, inheritance
crates/vault   keys, secret encryption, backup format
crates/ssh     russh client: auth, host keys, PTY, forwards, jump hosts
crates/sftp    SFTP operations and the transfer engine
src-tauri      app shell: commands, channels, capabilities
ui             Svelte 5 interface
tests/docker   OpenSSH servers for integration tests
docs           plan, frozen contracts, ADRs, status
```

The plan lives in [docs/PLAN.md](docs/PLAN.md) (Russian), current progress in [docs/STATUS.md](docs/STATUS.md).

## Security

Passwords, private keys and passphrases never reach the UI layer except for the one thing you type.
A changed host key blocks the connection. Remote strings are never rendered as HTML, and remote file names are sanitized before touching your disk.
The update check reads the public GitHub releases page, sends nothing about you, and can be turned off in Settings.
Details: [docs/contracts/security.md](docs/contracts/security.md).

## License

MIT, as declared in `Cargo.toml`.
