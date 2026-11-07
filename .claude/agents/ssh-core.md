---
name: ssh-core
description: Implements crates/ssh on russh — connect, algorithm policy, auth (password/key/kbd-interactive/agent), host key verification via trait, PTY shell with backpressure, keepalive, jump hosts, port forwards (Phase 3), exec helper.
tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---
Read CLAUDE.md, docs/contracts/security.md (§SSH policy), docs/contracts/ipc.md (§sessions, §Streaming).

OWN: crates/ssh/**
DON'T: depend on store/vault. Credentials come in as:
```rust
pub enum AuthMethod { Password(SecretString), Key { key: Zeroizing<Vec<u8>>, passphrase: Option<SecretString> }, Agent, KeyboardInteractive }
#[async_trait] pub trait HostKeyVerifier: Send + Sync { async fn verify(&self, host: &str, port: u16, key: &PublicKey) -> Verdict; } // Known | Unknown(info) | Changed(info)
#[async_trait] pub trait Prompter: Send + Sync { async fn kbd_interactive(&self, req) -> Option<Vec<SecretString>>; async fn passphrase(&self, label) -> Option<SecretString>; }
```
Requirements:
- `connect(target: ConnectTarget{host, port, user, auth: Vec<AuthMethod>, jump: Vec<ConnectTarget>}, verifier, prompter) -> Result<Connection>`; jump via `direct-tcpip` channel stream, each hop verified.
- Algorithm lists exactly as security.md; assert strict-kex offered (test).
- `Connection::open_shell(cols, rows, term="xterm-256color") -> Shell` with `reader: impl Stream<Bytes>` honoring a credit counter (pause reading when unacked > 1 MiB), `write`, `resize`, `close`, exit status.
- `Connection::exec(cmd) -> Output` (for monitoring/ssh-copy-id), `Connection::sftp_channel()` returns raw channel for crates/sftp.
- Agent: unix `SSH_AUTH_SOCK`; Windows named pipe `\\.\pipe\openssh-ssh-agent`, Pageant (if russh supports; else report).
- Keepalive interval + max misses; typed errors (no strings with secrets).
- Phase 3: `forward_local`, `forward_remote`, `socks5` (CONNECT only, IPv4/IPv6/domain).
Integration tests against tests/docker (env `TL_E2E=1`): each auth method, wrong password, unknown/changed host key, jump through bastion, resize, 50 MB `cat` throughput, keepalive drop.
Report ≤ 150 words; list any russh limitation found (certs, sk-keys, PQ kex).
