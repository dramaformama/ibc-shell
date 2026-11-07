---
name: tauri-bridge
description: Implements src-tauri — AppState, all IPC commands from ipc.md, session/transfer registries, raw-byte streaming with coalescing and credits, events, capabilities, CSP, tauri-specta bindings generation, auto-lock.
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/ipc.md (whole), docs/contracts/security.md (§WebView / Tauri, §Vault auto-lock).

OWN: src-tauri/** except src-tauri/gen/** and src-tauri/plugins/*-mobile/**
READ-ONLY: crates/*/src/lib.rs (public APIs only), docs/adr/ADR-004*.
DON'T: change crate internals — file a contract change request instead.

Requirements:
- `AppState { store, vault: RwLock<Vault>, sessions: DashMap<SessionId, SessionHandle>, transfers: TransferManager, pending: PendingPrompts }`. (`dashmap` allowed, justify.)
- Implement `HostKeyVerifier` + `Prompter` adapters: emit SessionEvent with pending_id, await reply with 120 s timeout.
- Resolve host → ConnectTarget: store.host_resolve + vault.with_secret; jump chain resolved recursively.
- Output pump: per session task reads Shell stream, coalesces (64 KiB / 8 ms), sends over Channel per ADR-004, tracks credits from shell_ack.
- `shell_write` accepts raw body (InvokeBody::Raw).
- Capabilities file: only listed plugins + our commands; CSP exactly as security.md; devtools off in release.
- Auto-lock timer, lock on system sleep where Tauri exposes it; on lock: close nothing, but new connects require unlock.
- `just bindings` target: specta export to ui/src/lib/api/bindings.ts; test that fails if bindings are stale.
- Remove mock usage via `VITE_API=real` default in prod build.
Tests: command-level tests with in-memory store + mock ssh (feature `test-mocks`).
Report ≤ 150 words.
