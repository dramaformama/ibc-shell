---
name: sftp-core
description: Implements crates/sftp — SFTP operations over an existing SSH channel and the transfer engine (queue, pipelining, resume, recursion, conflicts, remote-to-remote, path sanitization).
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/ipc.md (§SFTP), docs/contracts/security.md (§Terminal / remote-data hardening — file names).

OWN: crates/sftp/**
READ-ONLY: crates/ssh/src/lib.rs (public API).
DON'T: depend on store/vault/tauri.

Requirements:
- `Sftp::new(channel)`; ops: list (streamed for huge dirs, return Vec but read in batches), stat, lstat, mkdir -p, rename (posix-rename@openssh.com if available), remove (recursive, bounded concurrency 8), chmod, realpath, readlink, symlink.
- Transfer engine (`TransferManager`): jobs → files; max 3 files in parallel (config), per file 64 outstanding requests × 32 KiB; if server advertises `limits@openssh.com`, use its max-read/write-length. Progress events ≤ 4 Hz. Pause = stop issuing new requests; resume = continue from confirmed offset. Resume policy for existing smaller destination (compare size, optionally hash last 1 MiB). Preserve mtime (and mode when both sides POSIX).
- Remote→remote: stream through client with bounded buffer (8 MiB).
- Conflicts: `Ask` emits event and awaits reply via oneshot; "apply to all" stored per job.
- Local writes: sanitize name (security.md rules), write to `name.tlpart` then atomic rename; canonicalize and check destination is inside target dir. Don't follow remote symlinks on recursive download.
- Cancellation via `CancellationToken`.
Tests: unit for sanitizer (traversal, reserved names, NFC/NFD, > 255 bytes); e2e: 1 GiB sha256 match, resume after killed connection, 10 000 small files, unicode names, malicious names server.
Bench vs `sftp` CLI → numbers in report.
Report ≤ 150 words.
