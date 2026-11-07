---
name: ui-terminal
description: Builds the terminal view — xterm.js with WebGL, fit/resize, flow-control acks, tabs integration, search, copy/paste safety, themes, connection dialogs (host key TOFU, kbd-interactive, passphrase), reconnect banner, splits (Phase 3), mobile extra-keys bar (Phase 4).
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/ipc.md (§sessions, §Streaming), docs/contracts/ui.md (§Terminal screen, §Keyboard, §Mobile), docs/contracts/security.md (§Terminal hardening).

OWN: ui/src/lib/terminal/**, ui/src/routes/(app)/term/**
READ-ONLY: ui/src/lib/api/index.ts, ui/src/lib/stores/sessions.svelte.ts (public exports).

Requirements:
- Lazy-load xterm + addons. WebGL addon; on `onContextLoss` dispose and fall back to DOM renderer.
- Data path: Channel → `term.write(Uint8Array, () => api.shell_ack(id, n))`. Input: `onData`/`onBinary` → batched per rAF → `shell_write` raw bytes (UTF-8 encode).
- Resize: ResizeObserver + fit, debounce 50 ms, send only when cols/rows changed.
- OSC 52 off; OSC 8 links via confirm dialog; title sanitize; bracketed paste; multi-line paste confirm.
- Search (⌘F) overlay; font size zoom ⌘+/⌘-/⌘0; themes from ui.md.
- Dialogs: HostKey (fingerprint + randomart in mono, "Accept once" / "Accept & save"); HostKeyChanged red block, no accept; kbd-interactive (respect echo flag); passphrase.
- Reconnect banner, Enter reconnects.
- Phase 3 splits with own Splitter; broadcast mode with persistent warning badge.
- Phase 4 extra-keys bar (sticky Ctrl/Alt modifiers), IME composition handling test on iOS/Android (report issues; fallback hidden textarea strategy if needed).
Perf test: feed 100 MB via mock → main thread blocks < 100 ms (measure with PerformanceObserver longtask).
Report ≤ 150 words.
