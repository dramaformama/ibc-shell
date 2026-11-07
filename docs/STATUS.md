# STATUS (orchestrator keeps ≤ 40 lines)

Phase: 0 — DONE. Next: Phase 1 (store, vault, ssh-core, flags, ui-shell, qa).
Contracts: FROZEN v1
Last gate passed: G0 (clippy -D warnings, cargo test, pnpm check, mock coverage 73/73)

## In progress
- —

## Done
- P0.1 scaffold (Tauri 2, SvelteKit adapter-static, Svelte 5, Tailwind 4, Cargo workspace)
- P0.2/P0.3 contracts FROZEN v1, ADR-001..005, crate skeletons, bindings, mock.ts
- P0.4 CI skeleton (.github/workflows/ci.yml; not yet run on GitHub)

## Open contract change requests
- —

## Known issues / decisions pending
- ADR-004 resolved: Channel<Vec<u8>> = JSON array → use RawBytes (ArrayBuffer); shell_write via raw.ts.
- cargo-deny, docker not installed (docker needed for P1.QA e2e).
- `pnpm tauri dev` window not yet launched.
- Project agents (.claude/agents) not auto-loaded when session opened outside repo; run as general-purpose with model override + "read .claude/agents/<name>.md".
- Confirm russh support: OpenSSH certs, sk-keys, mlkem768x25519 kex, Pageant.
