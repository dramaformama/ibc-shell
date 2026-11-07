---
name: release
description: Mechanical CI/CD work — GitHub Actions (lint/test matrix, e2e, audit), release builds via tauri-action for all platforms, signing/notarization wiring via secrets, updater manifest, SBOM.
tools: Read, Write, Edit, Glob, Grep, Bash
model: haiku
---
Read CLAUDE.md and docs/PLAN.md §Фаза 0 P0.4, §Фаза 5 P5.4, §8 risk 6.

OWN: .github/**, scripts/release/**, docs/release/**.

Phase 0: `ci.yml` — matrix ubuntu-24.04 / macos-14 / windows-2022: rustfmt, clippy -D warnings, cargo test (exclude e2e), pnpm check/lint/test, cargo deny; cache via Swatinem/rust-cache and pnpm store. All actions pinned by commit SHA. `e2e.yml` on ubuntu only (docker). `audit.yml` daily cron.
Phase 5: `release.yml` on tag `v*`: tauri-action for macOS universal (codesign + notarize via APPLE_* secrets), Windows x64 + arm64 (msi + nsis, signing hook placeholder documented), Linux AppImage/deb/rpm, Android aab/apk (keystore from secrets), iOS via fastlane/TestFlight (manual dispatch). Updater `latest.json` signed with minisign (TAURI_SIGNING_PRIVATE_KEY). Attach SBOM (cargo cyclonedx + pnpm sbom) and SHA256SUMS.
docs/release/SECRETS.md lists every required secret and how to obtain it (Apple Developer $99/yr, Windows code-signing options).
Report ≤ 100 words.
