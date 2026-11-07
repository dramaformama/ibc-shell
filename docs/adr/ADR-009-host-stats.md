# ADR-009: Live host stats over the session connection

Status: accepted. Adds the IPC command `stats_sample` (passive for auto-lock) and the DTOs `HostStats`, `ProcRow`.

Decision: the dashboard samples a Linux host through a one-off exec channel on the terminal session's existing SSH connection (independent of the shell, no new login). A fixed read-only POSIX `sh` script (`/proc` twice, 1 s apart, `df`, `ps`) is run as `sh -c '...'`; output is capped at 256 KiB, deadline 8 s, parsed in pure Rust (`crates/ssh/src/stats_parse.rs`) and all server strings are sanitized.
Rules: nothing is installed on the server; only read-only commands; the UI polls (~2 s) only while the dashboard is visible and never overlaps requests; non-Linux hosts yield `Unsupported`.
Rejected: a remote agent (installs software), parsing the shell's output (fragile, disturbs the user), a separate connection (extra auth prompts).

## Amendment 2026-11-07: lighter sampling

Server load matters more than freshness. The UI waits 4 s after each answer (one sample ≈ 5 s, as in comparable tools) and pauses while the window is hidden. The script reads only `/proc` with `cat` on fast samples; `df`, `nproc`, `/etc/os-release` and `ps` run at most every 15 s per session (`full = true`), and the backend reuses the last full sample's slow fields in between (in-memory, dropped after 10 min unused). Additive DTO fields: `mem_free`, `ifaces: NetIface[]`, `full`.
