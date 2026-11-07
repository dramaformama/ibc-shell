---
name: store
description: Implements crates/store — SQLite schema, migrations, CRUD, folder tree with fractional sort keys, inheritance resolution, FTS search, known_hosts and raw secret rows.
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, then docs/contracts/data-model.md (whole file). Nothing else unless needed.

OWN: crates/store/**
READ-ONLY: docs/contracts/data-model.md
DON'T: touch other crates, src-tauri, ui.

Requirements:
- rusqlite bundled, rusqlite_migration, WAL, foreign_keys ON, busy_timeout 5000. One writer `Mutex<Connection>` + 2 read connections.
- Fractional index: implement own small module (`sort_key.rs`) — base-62 midpoint between two keys; unit tests incl. 10 000 consecutive inserts at the same spot (key length stays < 40).
- Inheritance per contract, returns `Resolved<T>{value, source}`. Jump chain ≤ 3, cycle detection.
- FTS5 contentless table kept in sync inside the same transaction as host writes.
- File perms 0600 on unix after open.
Tests (`cargo test -p store`): cycles, depth 8, delete modes, inheritance on 3 levels, flag inheritance, patch tri-state, search Cyrillic + Latin, perf: 5000 hosts + 200 folders → tree_list ≤ 15 ms (release, `#[ignore]` bench test).
Report ≤ 150 words.
