---
name: flags
description: Mechanical task — imports circle-flags SVGs, builds the flag registry (codes, aliases, RU/EN names via Intl.DisplayNames), the Flag component and FlagPicker data source, and generates the Rust KNOWN_FLAGS list.
tools: Read, Write, Edit, Glob, Grep, Bash
model: haiku
---
Read CLAUDE.md and docs/contracts/ui.md §FlagPicker only.

OWN: scripts/flags/**, ui/static/flags/**, ui/src/lib/flags/**, crates/store/src/flags_list.rs (generated file only), THIRD_PARTY_NOTICES.md (append).

Steps:
1. `scripts/flags/import.mjs`: download pinned release of HatScripts/circle-flags (MIT) tarball (version + sha256 pinned in script), copy country SVGs (ISO 3166-1 alpha-2 + `eu`, `un`, `xx` placeholder) to ui/static/flags/{cc}.svg, run SVGO (multipass). Report total size (target ≤ 400 KB for all files).
2. `ui/src/lib/flags/index.ts`: `FLAG_CODES`, `ALIASES` (uk→gb, usa→us, england→gb-eng if present, holland→nl, рф→ru, германия→de …), `flagName(code, locale)` via Intl.DisplayNames with fallback to code, `searchFlags(q, locale, recent)`.
3. `Flag.svelte`: props `code?: string | null`, `size = 16`, `title?`; renders `<img>` lazy; fallback grey circle with 2-letter code if file missing (onerror).
4. Generate `crates/store/src/flags_list.rs` with `pub const KNOWN_FLAGS: &[&str]`.
5. Add license notice.
Tests: vitest for alias resolution and search (Russian + English queries).
Report ≤ 100 words.
