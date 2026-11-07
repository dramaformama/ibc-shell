# AGENTS.md — iBC Shell

Start here (Codex and any other coding agent):

1. **`docs/HANDOFF.md`** — current state, unfinished work, where to continue, owner's preferences. Read it fully first.
2. **`CLAUDE.md`** — the project's hard rules. They apply to every agent, not only Claude:
   secrets never reach JS (except the one typed password); no secrets in logs/errors/Debug;
   no `unwrap`/`expect`/`panic!` outside tests in `crates/` and `src-tauri/`; host-key mismatch is a hard block;
   every IPC command is documented in `docs/contracts/ipc.md` first (contracts are frozen, change only via an ADR in `docs/adr/`);
   files ≤ 400 lines; `cargo deny` must pass.
3. `docs/STATUS.md` (short progress), the relevant section of `docs/PLAN.md`, `docs/TESTING.md` (manual checks).

Environment: `export PATH=/opt/homebrew/opt/rustup/bin:$PATH`. Gate before committing to `main`:
`cargo clippy --workspace --all-targets -- -D warnings && IBC_BIG_MIB=8 cargo test --workspace && cargo deny check && pnpm -C ui check && pnpm -C ui test && pnpm -C ui lint`.

The owner writes in Russian: reply in Russian, keep it short, write commit messages as plain Russian changelog lines, never `git push` without asking.
