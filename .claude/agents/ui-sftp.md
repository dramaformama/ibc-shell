---
name: ui-sftp
description: Builds the Termius-like SFTP screen — dual panes (local/remote, remote/remote), breadcrumbs, virtualized file table, DnD between panes and from OS, context menus, transfers drawer, conflict dialog; mobile single-pane variant (Phase 4).
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/ipc.md (§SFTP), docs/contracts/ui.md (§SFTP screen, §Mobile).

OWN: ui/src/lib/sftp/**, ui/src/routes/(app)/sftp/**
READ-ONLY: ui/src/lib/api/index.ts, shared components in ui/src/lib/components (import only).

Requirements:
- Pane = { endpoint, path, entries, selection, sort, filter, showHidden }. Each pane can switch to Local or any open SFTP session.
- Virtualized table (reuse VirtualList), columns: name, size (human, binary units), modified (Intl.DateTimeFormat), permissions (rwx string). Folders first.
- Breadcrumb editable on click; history back/forward.
- DnD pane→pane and OS→pane (Tauri drag-drop event gives local paths) → `transfer_enqueue`. Multi-select with ⇧/⌘.
- Context menu: Open, Download/Upload, Rename (F2), New folder, Delete (confirm with count), Copy path, Permissions dialog (octal + checkboxes).
- Transfers drawer: progress, speed (EMA smoothing), ETA, pause/resume/cancel/retry, clear finished; badge count in status bar.
- Conflict dialog per ui.md.
- Remote names rendered as text only; long names ellipsized with title tooltip.
Tests: vitest for sorting, size/permission formatting, selection logic.
Report ≤ 150 words.
