# ADR-005: Country flags as circle-flags SVGs
Date: 2026-11-07 · Status: Accepted

## Context
Folders and hosts show a country flag (sidebar 16 px, tabs 14 px, editor 20 px). Emoji regional-
indicator flags do not render on Windows (shown as two letters), and differ in style per OS.
Bundle size and offline operation matter; no external requests are allowed by the CSP.

## Decision
Use circle-flags (MIT) SVGs, optimized with SVGO, served as static files `ui/static/flags/{cc}.svg`
(lowercase ISO 3166-1 alpha-2 plus circle-flags extras such as `eu`, `un`, `xx`), rendered with
`<img loading="lazy" decoding="async" width height>`. Country names via `Intl.DisplayNames`
(ru/en), no data bundle. Accepted codes are generated into `crates/store/src/flags_list.rs`
(`KNOWN_FLAGS`, placeholder until P1.FLAGS). Fallback: grey circle with the code.

## Consequences
Identical look on every OS; ~250 small files in the bundle (lazy-loaded). Licence notice in
THIRD_PARTY_NOTICES.md. Contracts: data-model.md invariant 6, ui.md §FlagPicker.
