# STATUS (orchestrator keeps ≤ 40 lines)

Name: ibc server (crates ibc-*, id app.ibcserver). Phase 1 DONE (G1 passed). Next: Phase 2.
Contracts: FROZEN v1

## Done
- P0: scaffold, contracts FROZEN v1, ADR-001..005, mock API, CI skeleton
- P1: store (22 tests, tree_list 5000 hosts ≈2.2ms), vault (13), ssh (23 unit + 8/8 docker e2e), flags (19), ui-shell (32 tests, light default theme), docker test servers (2201-2205)
- Security review vault+ssh: no blockers; MAJOR/MINOR fixed (ring instead of aws-lc, handshake timeout, etc.)
- deny.toml: documented ignores (rsa Marvin, paste, proc-macro-error)

## Known issues / decisions pending
- paraglide compile fetches plugin from CDN (offline builds fail) — check/pin in Phase 2.
- UI not verified: real-pointer DnD, context menu, settings, FlagPicker screenshots. FlagPicker ~270 rows unvirtualized.
- UI missing: Forwards section, Snippets/Keys/Known hosts pages (Phase 3).
- russh: no sk-keys, certificates not wired; keyboard-interactive answers can't be zeroized.
- CI not yet run on GitHub; repo URL unknown (needed for README download links).
- Project agents not auto-loaded: spawn general-purpose + model + "read .claude/agents/<name>.md".
