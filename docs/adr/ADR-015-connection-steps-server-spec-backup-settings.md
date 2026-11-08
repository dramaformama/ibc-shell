# ADR-015: connection steps, server configuration, settings in backups

Status: accepted, 2026-11-08. Amends ADR-004 (events), ADR-009 (HostStats), ADR-007 (backup).

## Context
1. A failed connect only said "server unavailable": no way to tell DNS from a closed port, a dropped
   handshake from a refused login. A dropped session only said "connection closed".
2. Server cards and the Status page showed load but not what the server is (cores, CPU, RAM, disk).
3. The `.tlvault` backup carried servers and secrets but no settings, and Settings had no way to
   save or restore it, so a new computer could not be set up from one file.

## Decision
**Connection steps.** `connect` now reports the steps in the order they really run: `Resolving`
(DNS only), then `Connecting` (TCP, per address), `Handshake` (key exchange and host key check, up
to the `none` probe), `Authenticating`, `OpeningChannel`. New `SessionEvent::Attempt { hop, addr,
failure }` is sent for every TCP attempt (`failure` = `null`, `Timeout` or the IO error kind), so
the UI can show which address answered. The UI keeps a per-connect log (step, hop, time without
prompts, attempts) and maps "step + error" to one explanation; it can copy a plain-text report
(host labels, addresses, error codes; no secrets). `Prompter::attempt` is a default method, so other
implementors are unaffected.

**Why a session ended.** `Disconnected.reason` is now a code: `closed` (by the user), `shell_exited`,
`keepalive`, `inactivity`, `server:<DisconnectCode>`, `network:<IoKind>`, `protocol`. Codes come from
russh's disconnect callback and IO kinds; the server's own text is never forwarded. The UI localizes
them and shows an unknown value as is.

**Server configuration.** `HostStats` gains slow fields `cpu_model` (`/proc/cpuinfo`, else `lscpu`),
`disk_dev_bytes` (size of the physical disk that holds `/`) and `disk_kind` (`NVMe`, `SSD`, `HDD` as
the guest reports it; virtual disks may say SSD for anything). They are read with the other slow
fields and remembered with the last-seen snapshot, so cards show them before a connection is up.

**Settings in backups.** The backup payload gains `settings: [{k, v}]` (JSON text), optional on read,
so old backups still import and old apps ignore the field; the format version stays 1. Excluded:
`stats.last` (a cache) and `security.*` (bound to one computer's vault). On import the keys must be
`[a-z0-9_.-]{1,64}`, values valid JSON ≤ 64 KiB, at most 512 entries; accepted keys replace the
current ones. The webview's `localStorage` preferences (`ibc.*`, language) are saved by the UI as the
setting `ui.local` just before export and restored from it after import, then the page reloads.
`ImportSummary` gains `settings: u32`. Settings gets a "Backup" section for both directions.

## Consequences
- The IPC contract changes: new event variant, `HostStats` fields, `ImportSummary.settings`,
  documented in `ipc.md`; bindings regenerated.
- The stats script reads two more small files per full sample (every 15 s).
- A restore replaces local settings with the file's; servers are merged (existing ids stay).
- The backup password still cannot be recovered; the file holds every secret.
