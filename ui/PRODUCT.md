# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Tauri 2 (WebView) + SvelteKit (adapter-static) + Svelte 5 runes + Tailwind CSS v4 + bits-ui (headless). Frozen in CLAUDE.md; mobile (iOS/Android) via the same UI in Phase 4.

## Users

People who administer many Linux servers over SSH (VPN nodes, VPS across countries): developers, sysadmins, self-hosters. The author is the first user. Distributed as an open-source project on GitHub; anyone who needs it downloads it. No accounts, no cloud.

## Product Purpose

Cross-platform SSH + SFTP client (macOS, Windows, Linux, later iOS/Android). Success: connect to the right server in two keystrokes, move files without friction, and trust that secrets never leave the machine.

## Positioning

Termius-class workflow (terminal tabs, two-pane SFTP, vault) that is light (small installer, low RAM), organizes servers in nested folders with country flags (like the Remnawave hosts list), and is transparent about security (host-key prompts, visible vault state, no secrets in the UI).

## Operating Context

Desktop first: sidebar tree of folders/hosts + tabbed content (terminal, SFTP). Command palette (⌘K). Long sessions in a terminal; dark theme default, light theme supported. Russian and English UI.

## Capabilities and Constraints

Contracts in docs/contracts/ui.md and ipc.md are FROZEN v1 (tokens, layout, components, keyboard). UI develops against ui/src/lib/api/mock.ts. Secrets never reach JS except one typed password/passphrase. Remote strings are never rendered as HTML. No heavy component libs, no CSS-in-JS, virtualize lists > 200 rows. Open decision: project name (working name "Termlite").

## Brand Commitments

Binding from the owner: light, laconic, minimalist, premium-product feel. Country flags are circle-flags SVG (not emoji). Do not copy Termius name, icons, or assets.

## Evidence on Hand

No users, testimonials, or benchmarks yet. Flag assets come from circle-flags (MIT). Do not fabricate proof or metrics.

## Product Principles

1. Lightness over features: quiet chrome, content first, terminal is the hero.
2. Structure by place: folders and flags are the primary way to find a server.
3. Security is visible, not loud: state of vault and host keys is always legible, never alarming by default, hard-blocking when it matters.
4. Everything reachable by keyboard.
5. Honest and open: nothing hidden, nothing phoning home.

## Accessibility & Inclusion

Keyboard-complete, visible focus, prefers-reduced-motion respected, contrast AA in both themes, RU + EN strings via paraglide.
