# ADR-002: russh + russh-sftp for SSH and SFTP
Date: 2026-11-07 · Status: Accepted

## Context
Need a pure-Rust async SSH client usable on all five targets (no OpenSSL / libssh2 FFI on mobile),
with modern KEX (strict-kex against Terrapin, post-quantum hybrid), agent auth and SFTP over the
same connection. Licences must pass `cargo deny` (OpenSSL licence is not on the allow list).

## Decision
`russh 0.65.x` with `default-features = false, features = ["ring", "rsa", "flate2"]` (ring backend
instead of default aws-lc-rs, whose aws-lc-sys carries the OpenSSL licence and needs cmake/NASM on
Windows), plus `russh-sftp 3.x`. Verified in 0.65.1 source: `mlkem768x25519-sha256` (via RustCrypto
`ml-kem`), `kex-strict-c-v00@openssh.com`, OpenSSH certificate auth (`authenticate_openssh_cert`),
agent client (`keys::agent`). Algorithm policy = security.md §SSH policy, enforced in `crates/ssh`.

## Consequences
FIDO/sk keys, GSSAPI and X11 are out of scope for v1. If ring lacks a cipher we need, switching to
aws-lc-rs requires a new ADR (licence exception). `ssh`/`sftp` crates stay free of store/vault
deps (ipc.md §crates/ssh, §crates/sftp public API).
