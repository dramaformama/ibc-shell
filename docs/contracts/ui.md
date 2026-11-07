# Contract: UI
Status: `FROZEN v1` (2026-11-07). Changes only via ADR, bump to `v1.x`. Flags: ADR-005.

## API access
UI code imports only `$lib/api` (`api: Api`, `apiEvents: ApiEvents`, all DTO types). `VITE_API=real` → Rust bridge
(`bindings.ts` + `raw.ts`), otherwise the in-memory mock (`mock.ts`: seed folders de/nl/fi + nested, 12 hosts,
2 identities; vault password `ibc-server`, `?vault=new` = onboarding; echo terminal with `help`, `colors`, `flood`, `exit`;
hosts labelled "new key" / "changed key" / "2FA" exercise TOFU, hard block, keyboard-interactive). Never import
`bindings.ts` directly from components.

## Design principles
Minimal, quiet chrome, content first. One accent color. No gradients, no heavy shadows. Motion ≤ 150 ms, `prefers-reduced-motion` respected. Everything reachable by keyboard.

## Tokens (Tailwind v4 `@theme`, CSS variables)
- Font UI: system stack (`-apple-system, "Segoe UI Variable", "Segoe UI", Inter, sans-serif`). Mono: bundled **JetBrains Mono** (OFL, woff2 subset Latin+Cyrillic, ~90 KB) — user can choose any installed font.
- Sizes: 12 / 13 (base) / 15 / 18. Radius: 6 (controls), 10 (cards/dialogs). Spacing 4-pt grid.
- Dark (default): bg `#0E0F12`, surface `#16181D`, surface-2 `#1D2026`, border `#262A31`, text `#E6E8EB`, muted `#8B929C`, accent `#5B8CFF`, ok `#3CCB7F`, warn `#F5B547`, danger `#F06161`.
- Light: bg `#FAFAFB`, surface `#FFFFFF`, surface-2 `#F2F3F5`, border `#E3E5E8`, text `#16181D`, muted `#6B7280`, same accents slightly darker.
- Terminal themes shipped: ibc server Dark, ibc server Light, Solarized Dark/Light, Dracula-like "Night" (own palette, no trademark), Nord-like "Frost" (own palette). Theme = 16 ANSI + fg/bg/cursor/selection.

## Layout (desktop)
```
┌ Titlebar (custom on mac: traffic lights inset; native on win/linux) ───────────────┐
│ Sidebar 260px (resizable 200–420, collapsible ⌘B) │ Tab bar                          │
│  [Search ⌘K]                                      │ ┌ tab: 🇩🇪 de-fra-1  ● ×  ┐ +    │
│  ▾ (de) Germany              3                    │ Content: Terminal | SFTP | Editor│
│     (de) fra-1     root@1.2.3.4                   │                                  │
│     (de) fra-2                                    │                                  │
│  ▾ (nl) Netherlands                               │                                  │
│  ▸ Personal                                       │ Status bar (optional): latency,  │
│  ── Snippets · Keys · Known hosts · Settings ──   │ forwards, transfers              │
└───────────────────────────────────────────────────┴──────────────────────────────────┘
```
- `(de)` = circle flag 16 px (sidebar), 14 px (tabs), 20 px (editor/picker). Flag left of name, like Remnawave hosts list.
- Host row: flag · label · muted `user@hostname` on hover or when width allows · status dot if a session is open.
- Folder row: chevron · flag (if set, else folder icon or custom lucide icon) · name · count (muted).
- Host inherits folder flag when its own is NULL; inherited flag rendered at 70% opacity in editor only (not in list).

## Components (bits-ui headless + own styles)
`Button, IconButton, Input, Select, Combobox, Dialog, Popover, Tooltip, ContextMenu, DropdownMenu, Tabs, Switch, Toast, Tree (own, virtualized), VirtualList (own), Flag, FlagPicker, KbdHint, Splitter, CommandPalette`.

### FlagPicker
Combobox: search by ISO code, English name, Russian name (`Intl.DisplayNames(['ru','en'], {type:'region'})`), alias (`uk→gb`, `usa→us`, `holland→nl`). Sections: Recent (last 8), All. "None" option. Optional (Phase 3, `crates/geo`): "Auto-detect from IP" button.

### Host editor (sheet on the right, 420 px; full screen on mobile)
Sections: General (label, hostname, port, flag, folder, tags) · Auth (username, identity picker / create inline) · Advanced (jump host, startup command, keepalive, terminal theme, legacy algorithms) · Forwards · Notes.
Every inheritable field shows placeholder `inherited from <Folder name>: value`, with "reset to inherit" button.

### Command palette (⌘K / Ctrl+K)
Fuzzy (own tiny scorer, no fuse.js): hosts (connect / open SFTP), folders (jump to), snippets (paste into active terminal), actions (new host, new folder, lock vault, settings, toggle theme). Quick connect: typing `user@host:port` offers "Connect to …".

## Keyboard (mac ⌘ / win-linux Ctrl+Shift in terminal context)
New tab ⌘T · Close ⌘W · Next/prev tab ⌃Tab / ⌃⇧Tab · Tab N ⌘1…9 · Palette ⌘K · Find in terminal ⌘F · Sidebar ⌘B · Split (Phase 3) ⌘D / ⌘⇧D · SFTP for current host ⌘⇧S · Lock ⌘L · Settings ⌘,

## Sidebar tree behaviour
- DnD: hosts and folders; drop above/below/inside with 6 px indicator; auto-expand folder after 600 ms hover; multi-select with ⇧/⌘ and drag many.
- Keyboard: ↑↓ move, ←→ collapse/expand, Enter connect, ⇧Enter SFTP, F2 rename, Del delete (confirm).
- Context menu: Connect · Open SFTP · Duplicate · Edit · Set flag ▸ · Move to ▸ · Delete. Folder: New host here · New subfolder · Set flag ▸ · Edit defaults · Delete.
- Virtualized when total visible rows > 200.

## SFTP screen
Two panes (left local by default on desktop, right remote; either side switchable to any open SFTP session). Each: breadcrumb (editable path), toolbar (up, refresh, new folder, hidden files toggle, search filter), virtualized table (icon, name, size, modified, permissions). Drag between panes or from OS (desktop drop events) → `transfer_enqueue`. Transfers drawer at bottom: list with progress bar, speed, ETA, pause/resume/cancel/retry, "clear finished".
Conflict dialog: file name, both sizes & dates, choices Overwrite / Skip / Rename / Resume, checkbox "Apply to all".

## Terminal screen
xterm fills area; padding 8 px; scrollback default 10 000 lines (setting). Reconnect banner on disconnect (Enter = reconnect). Host key / kbd-interactive / passphrase dialogs are modal over the terminal tab only (not the whole app).

## Mobile (Phase 4)
- Root: host list (folders as collapsible sections, flags), search on top, FAB "+".
- Terminal: full screen, extra-keys bar above keyboard (Esc Tab Ctrl Alt ← ↑ ↓ → | / ~ - and a snippets key), long-press = paste/select, pinch = font size, two-finger swipe = scrollback.
- SFTP: single pane, bottom sheet for actions, upload via system document picker, download → share sheet / Files.

## i18n
paraglide-js, `ui/messages/{en,ru}.json`. No hard-coded strings in components. Pluralization via Intl.PluralRules.

## Performance rules
- Initial route JS ≤ 200 KB gzip; xterm and SFTP screens lazy-loaded.
- Tree diffing by ids; no deep reactive proxies for 5000 hosts — use `$state.raw` + immutable updates.
- Flags: `<img src="/flags/{cc}.svg" loading="lazy" decoding="async" width height>`; no inline SVG duplication.
