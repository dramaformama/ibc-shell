---
name: mobile
description: Phase 4 — iOS/Android targets: tauri mobile init, native secure-storage plugins (iOS Keychain, Android Keystore), biometric unlock, lifecycle (Android foreground service, iOS background handling), FLAG_SECURE / snapshot blur, build configs.
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/security.md (§Vault, biometric), docs/PLAN.md §Фаза 4 and §8 risks 1–3.

OWN: src-tauri/gen/**, src-tauri/plugins/*-mobile/**, mobile sections of src-tauri/tauri.conf.json (via a separate `tauri.ios.conf.json` / `tauri.android.conf.json`).
READ-ONLY: crates/vault/src/keystore.rs (KeyStore trait).

Requirements:
- Android plugin (Kotlin): Keystore AES-256-GCM key, `setUserAuthenticationRequired(true)` variant for biometric; wraps/unwraps DEK; implements KeyStore trait through Tauri plugin bridge. minSdk 26.
- iOS: verify `keyring` iOS support for pinned version; if insufficient, Swift plugin using `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` + `SecAccessControl(.biometryCurrentSet)` for biometric item.
- Android foreground service while ≥ 1 session open, persistent notification "N active sessions" with Disconnect all action. Pick FGS type and write justification text for Play review in docs/release/play-fgs.md.
- iOS: on background → mark sessions "suspended"; on foreground → auto-reconnect if was connected < 10 min ago.
- FLAG_SECURE on vault/identity screens (Android); blur overlay on `applicationWillResignActive` (iOS).
- Status bar/safe-area insets passed to UI as CSS env vars.
Report ≤ 150 words with exact device/OS versions tested.
