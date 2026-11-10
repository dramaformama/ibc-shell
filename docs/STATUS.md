# STATUS (orchestrator keeps ≤ 40 lines)

Name: iBC Shell (crates ibc-shell-*, id app.ibcshell). Phases 1-2 DONE; UI redesign (Apple style, server-grouped tabs, simple login/password, output highlighting, resizable SFTP columns, update check) DONE.
DONE: Linux server metrics: stats_sample, Overview cards and terminal inspector (persisted visibility), shared polling, nullable samples. Mock UI checked in light/dark; real Linux acceptance remains. See docs/HANDOFF.md.
DONE: Server-first Overview includes saved hosts, independent TCP reachability (4 concurrent), search/filter, virtualized grid. Shared Apple-style controls, grouped settings, persistent SFTP views + SSH reuse; ADR-010.
DONE 1.0.0 (ADR-017): local terminal in sidebar, tab drag-reorder, terminal folder → Files + drop upload, activity marks + background-command notifications, rusqlite 0.41.
Open security findings: #6-#9, #11-#14 in docs/security-review.md (#5, #10 closed).
Contracts: FROZEN v1.1 (ADR-010)

## Done
- P0: scaffold, contracts FROZEN v1, ADR-001..005, mock API, CI skeleton
- P1: store (22 tests, tree_list 5000 hosts ≈2.2ms), vault (13), ssh (23 unit + 8/8 docker e2e), flags (19), ui-shell (32 tests, light default theme), docker test servers (2201-2205)
- Security review vault+ssh: no blockers; MAJOR/MINOR fixed (ring instead of aws-lc, handshake timeout, etc.)
- deny.toml: documented ignores (rsa Marvin, paste, proc-macro-error)

## Known issues / decisions pending
- paraglide compile fetches plugin from CDN (offline builds fail) — check/pin in Phase 2.
- UI not verified: real-pointer DnD, context menu, settings, FlagPicker screenshots. FlagPicker ~270 rows unvirtualized.
- UI missing: Forwards section (backend `start_forward` is a stub), tags (CRUD only, no UI).
- russh: no sk-keys, certificates not wired; keyboard-interactive answers can't be zeroized.
- CI not yet run on GitHub; repo URL unknown (needed for README download links).
- Project agents not auto-loaded: spawn general-purpose + model + "read .claude/agents/<name>.md".
