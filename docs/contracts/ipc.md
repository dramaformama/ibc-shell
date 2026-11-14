# Contract: IPC (src-tauri ↔ ui)
Status: `FROZEN v1.1` (2026-11-07). Changes only via ADR (`docs/adr/`), bump to `v1.x`.

Commands are `#[tauri::command] #[specta::specta] async fn` returning `Result<T, ApiError>`.
TS bindings are generated (`just bindings` → `ui/src/lib/api/bindings.ts`, never edited by hand);
every command returns `{status:"ok", data} | {status:"error", error: ApiError}`
(`ErrorHandlingMode::Result`). TS accessor = camelCase of the command (`tree_list` → `treeList`);
argument names are camelCase in TS, snake_case in Rust. DTO fields stay snake_case.
Numbers: `i64` unix millis and `u64` byte counts are exported as `number` (< 2^53, ADR-004).
IDs are UUID v7 strings. UI code imports only `$lib/api` (`api`, `apiEvents`, types).

## DTO sources (exact Rust definitions = the contract)
| DTO (TS name) | Rust |
|---|---|
| `Tree`, `Folder`, `FolderDefaults`, `FolderInput`, `FolderPatch`, `FolderDefaultsPatch`, `Host`, `HostInput`, `HostPatch`, `NodeRef`, `FolderDeleteMode` | `crates/store/src/tree.rs` |
| `IdentityKind`, `Tag`/`TagInput`/`TagPatch`, `Snippet`/`SnippetInput`/`SnippetPatch`, `PortForward`/`ForwardInput`/`ForwardPatch`/`ForwardKind`, `KnownHostSummary` | `crates/store/src/catalog.rs` |
| `ResolvedHost`, `Resolved<T>`, `Source` | `crates/store/src/resolve.rs` |
| `*Id` (FolderId, HostId, IdentityId, TagId, SnippetId, ForwardId, KnownHostId) | `crates/store/src/ids.rs` |
| `VaultState`, `VaultMode`, `ImportSummary` | `crates/vault/src/lib.rs` |
| `HostKeyInfo`, `HostKeyMismatch`, `ConnectStep`, `KbdPrompt`, `ForwardState` | `crates/ssh/src/{hostkey,config,session}.rs` |
| `DirListing`, `Entry`, `EntryKind`, `TransferEvent`, `TransferId`, `ConflictId`, `ConflictPolicy`, `ConflictChoice`, `ConflictInfo` | `crates/sftp/src/{model,transfer}.rs` |
| `HostReachability` | `src-tauri/src/commands/probe.rs` (ADR-010) |
| `ApiError` | `src-tauri/src/api/error.rs` |
| `SessionId`, `SftpId`, `PendingId`, `IdentityDto`, `IdentityInput`, `IdentityPatchInput`, `VaultStatus`, `QuickConnect`, `SessionEvent`, `SftpTarget`, `Endpoint`, `TransferRequest`, `AppInfo` | `src-tauri/src/api/dto.rs` |
| `VaultLocked`, `TreeChanged`, `UpdateAvailable`, `TrayOpenHost` (events) | `src-tauri/src/api/events.rs` |

Patch DTOs (`*Patch`): field absent = untouched, `null` = clear (inherit), value = set (`data-model.md` §Patch semantics).
Secret inputs (`password`, `private_key_pem`, `passphrase`, kbd `answers`) are `SecretInput`/`SecretString` in Rust
(TS `string`), redacted `Debug`, never returned.

## Errors
```rust
#[serde(tag = "code", content = "detail")]
pub enum ApiError {
  VaultLocked, WrongPassword, NotFound, Validation(String), Cycle, TooDeep, JumpCycle,
  HostKeyUnknown(Box<HostKeyInfo>),     // non-interactive paths only; interactive = SessionEvent::HostKeyPrompt
  HostKeyChanged(Box<HostKeyMismatch>), // red block, NO accept route in v1 (only delete in Known hosts)
  AuthFailed { methods_left: Vec<String> },
  Network(String), Timeout, Sftp(String), Cancelled, Conflict(Box<ConflictInfo>),
  Unsupported,                          // e.g. local_* on mobile
  Internal(String),                     // sanitized; Phase 0 stubs return Internal("not implemented")
}
```

## Commands — store / tree
| command | args | returns |
|---|---|---|
| `tree_list` | — | `Tree` |
| `folder_create` | `input: FolderInput` | `Folder` |
| `folder_update` | `id: FolderId, patch: FolderPatch` | `Folder` |
| `folder_move` | `id: FolderId, parent: FolderId \| null, after: NodeRef \| null` (null = first) | `null` |
| `folder_delete` | `id: FolderId, mode: FolderDeleteMode` | `null` |
| `host_create` | `input: HostInput` | `Host` |
| `host_update` | `id: HostId, patch: HostPatch` | `Host` |
| `host_duplicate` | `id: HostId` | `Host` |
| `host_delete` | `ids: HostId[]` | `null` |
| `host_move` | `id: HostId, folder: FolderId \| null, after: NodeRef \| null` | `null` |
| `host_resolve` | `id: HostId` | `ResolvedHost` |
| `host_search` | `q: string, limit: number` | `HostId[]` |
| `identity_list` | — | `IdentityDto[]` |
| `identity_create` | `input: IdentityInput` (secrets accepted ONCE → vault) | `IdentityDto` |
| `identity_update` | `id: IdentityId, patch: IdentityPatchInput` | `IdentityDto` |
| `identity_delete` | `id: IdentityId` | `null` |
| `identity_import_file` | `passphrase: string \| null` (backend opens the file dialog, ADR-006) | `IdentityDto \| null` (null = cancelled) |
| `tag_list` | — | `Tag[]` |
| `tag_create` | `input: TagInput` | `Tag` |
| `tag_update` | `id: TagId, patch: TagPatch` | `Tag` |
| `tag_delete` | `id: TagId` | `null` |
| `snippet_list` | `folder_id: FolderId \| null` (global + that folder) | `Snippet[]` |
| `snippet_create` | `input: SnippetInput` | `Snippet` |
| `snippet_update` | `id: SnippetId, patch: SnippetPatch` | `Snippet` |
| `snippet_delete` | `id: SnippetId` | `null` |
| `forward_list` | `host_id: HostId \| null` | `PortForward[]` |
| `forward_create` | `input: ForwardInput` | `PortForward` |
| `forward_update` | `id: ForwardId, patch: ForwardPatch` | `PortForward` |
| `forward_delete` | `id: ForwardId` | `null` |
| `known_host_list` | — | `KnownHostSummary[]` |
| `known_host_delete` | `id: KnownHostId` | `null` |
| `settings_get` | `key: string` | JSON value \| `null` |
| `settings_set` | `key: string, value: JSON` | `null` |

`IdentityDto { id, label, username?, kind, key_algo?, key_fingerprint?, public_key?, has_password, has_passphrase, created_at, updated_at }` — no secrets, no secret ids.

## Commands — vault
| command | args | returns |
|---|---|---|
| `vault_status` | — | `VaultStatus { state: "Uninitialized"\|"Locked"\|"Unlocked", mode: "OsKeychain"\|"MasterPassword"\|null, biometric_available }` |
| `vault_init` | `mode: VaultMode, password: string \| null` | `null` (vault left unlocked) |
| `vault_unlock` | `password: string \| null` (null = keychain / biometric) | `null` |
| `vault_lock` | — | `null` (+ event `vault://locked`) |
| `vault_change_password` | `old_password: string, new_password: string` | `null` |
| `vault_remove_password` | `password: string` (current master password; ADR-014) | `null` (vault moved to `OsKeychain` mode and left unlocked) |
| `vault_export` | `password: string` (≥ 10 chars; backend opens the save dialog, ADR-007) | `boolean` (false = cancelled) |
| `vault_import` | `password: string` (backend opens the file dialog, ADR-007) | `ImportSummary \| null` (null = cancelled; + event `tree://changed`; also restores settings, `ImportSummary.settings`, ADR-015) |

## Commands — sessions (SSH)
| command | args | returns |
|---|---|---|
| `session_open` | `host_id, cols: u16, rows: u16, on_data: Channel<ArrayBuffer>, on_event: Channel<SessionEvent>` | `SessionId` |
| `session_open_adhoc` | `target: QuickConnect {user?, hostname, port?}, cols, rows, on_data, on_event` | `SessionId` |
| `session_open_local` | `cols, rows, on_data, on_event` (desktop only: the user's login shell in a local PTY, ADR-017; `Connected` → output; shell exit → `ExitStatus` + `Disconnected{shell_exited, can_reconnect: true}`; `session_reconnect` restarts it) | `SessionId` |
| `shell_write` | RAW: body = bytes (`InvokeBody::Raw`), header `x-session-id` (TS: `api.shellWrite(sessionId, Uint8Array)`) | `null` |
| `shell_ack` | `session_id, bytes: u32` | `null` |
| `shell_resize` | `session_id, cols: u16, rows: u16` | `null` |
| `session_close` | `session_id` (also cancels its pending prompts) | `null` |
| `session_reconnect` | `session_id` (reuses channels) | `null` |
| `hostkey_accept` | `pending_id, remember: bool` (unknown keys only; reject = `session_close`) | `null` |
| `kbd_interactive_reply` | `pending_id, answers: string[] \| null` (null = cancel) | `null` |
| `passphrase_reply` | `pending_id, passphrase: string \| null` (null = cancel) | `null` |

`session_open*` resolves once the session task is spawned; everything else arrives on `on_event`.
`SessionEvent` (tag `type`): `Connecting{step: ConnectStep, hop}`, `Attempt{hop, addr, failure: string|null}` (one TCP attempt at `ip:port`; `failure` = `Timeout` or the IO error kind; ADR-015), `HostKeyPrompt{pending_id, info: HostKeyInfo}`,
`KbdInteractive{pending_id, name, instruction, prompts: KbdPrompt[]}`, `PassphrasePrompt{pending_id, identity_label}`,
`Connected{server_banner: string|null}`, `Failed{error: ApiError}` (before Connected: host key changed, auth, network),
`Disconnected{reason, can_reconnect}` (`reason` is a code: `closed`, `shell_exited`, `keepalive`, `inactivity`, `server:<DisconnectCode>`, `network:<IoKind>`, `protocol`; ADR-015), `ExitStatus{code}`, `ForwardStatus{forward_id, state: ForwardState, error|null}`.
`ConnectStep`: `Resolving` (DNS) `| Connecting` (TCP) `| Handshake` (key exchange, host key) `| Authenticating | OpeningChannel`, in that order (ADR-015).

### Streaming & backpressure (normative, ADR-004)
- Rust → UI: `on_data: Channel<RawBytes>`; `RawBytes` sends `InvokeResponseBody::Raw` → JS `ArrayBuffer` (TS `Channel<ArrayBuffer>`). `Channel<Vec<u8>>` is FORBIDDEN (JSON number array). Coalesce: flush when buffer ≥ 64 KiB or 8 ms elapsed.
- Credit window: Rust stops reading the SSH channel when unacked bytes > 1 MiB (`ibc_shell_ssh::CREDIT_WINDOW`). UI calls `shell_ack(bytes)` from xterm `write(data, callback)`.
- UI → Rust: keystrokes batched per animation frame, sent via raw `shell_write`.

### crates/ssh public API (bridge uses; `crates/ssh/src/lib.rs`)
`connect(chain: Vec<Hop>, &impl KnownHosts, &impl Prompter) -> Result<Connection, SshError>` (jump hosts first, target last, ≤ 4 hops).
`Connection { server_banner(), open_shell(PtyRequest) -> Shell, exec(cmd, max_output) -> ExecOutput, open_sftp() -> SubsystemStream (AsyncRead+AsyncWrite), start_forward(ForwardSpec) -> ForwardHandle, disconnect() }`.
`Shell { read() -> Option<ShellOutput::{Data, ExitStatus}>, write(&[u8]), resize(cols, rows), ack(u32), close() }`.
`KnownHosts { lookup(pattern) -> Vec<KnownKey>, add(pattern, &KnownKey) }` (pattern via `host_pattern(host, port)`).
`Prompter { progress(hop, ConnectStep), confirm_unknown_host(HostKeyInfo) -> HostKeyDecision, keyboard_interactive(req) -> Option<Vec<SecretString>>, passphrase(label) -> Option<SecretString> }`.
`AuthMethod::{Password, Key{pem, passphrase, identity_label}, Agent, KeyboardInteractive}` (redacted Debug).
`SshError::{HostKeyChanged, AuthFailed{methods_left}, Network, Timeout, Cancelled, Negotiation, InvalidChain, KnownHosts, Protocol}`.

## Commands — host windows (ADR-019)
| command | args | returns |
|---|---|---|
| `host_window_open` | `host_id: HostId, view: HostWindowView, transfer: HostWindowTransfer \| null` (desktop, unlocked vault, main window) | `string` (window label) |

| `host_window_pause` | `id: SessionId, pause: bool` (owner only) | `null` |
| `host_window_take` | `on_data: Channel<ArrayBuffer>, on_event: Channel<SessionEvent>` (host window) | `HostWindowTransfer \| null` |
| `host_window_transferred` | `label: string, cancel: bool` (source window) | `boolean` |

`HostWindowTransfer` carries the existing session IDs, terminal display snapshot and pane paths in memory. The source owns the sessions until the destination takes them.

## Commands — SFTP
| command | args | returns |
|---|---|---|
| `sftp_open` | `target: SftpTarget ({kind:"session", session_id} \| {kind:"host", host_id}), on_event: Channel<SessionEvent>` | `SftpId` |
| `sftp_close` | `sftp_id` | `null` |
| `sftp_list` | `sftp_id, path` | `DirListing { path, entries: Entry[] }` |
| `sftp_stat` | `sftp_id, path` | `Entry { name, kind: "file"\|"dir"\|"symlink"\|"other", size, mtime: ms\|null, mode, link_target\|null }` |
| `sftp_mkdir` | `sftp_id, path` | `null` |
| `sftp_rename` | `sftp_id, from, to` | `null` |
| `sftp_remove` | `sftp_id, path, recursive: bool` | `null` |
| `sftp_chmod` | `sftp_id, path, mode: u32` | `null` |
| `sftp_realpath` | `sftp_id, path` (`"."` = home) | `string` |
| `file_preview` | `endpoint: Endpoint, path: string, tail: bool, blocks: u8` (unlocked vault, ADR-020) | `FilePreview { text: string, truncated: bool }` |
| `file_open` | `endpoint: Endpoint, path: string` (desktop, unlocked vault, ADR-018) | `null` |
| `local_home` | — (registers home as allowed root; desktop only) | `string` |
| `local_list` | `path` | `DirListing` |
| `local_mkdir` | `path` | `null` |
| `local_rename` | `from, to` | `null` |
| `local_remove` | `path, recursive: bool` | `null` |
| `transfer_enqueue` | `request: TransferRequest { src: Endpoint, dst: Endpoint, items: string[], dst_dir, conflict: ConflictPolicy }` | `TransferId[]` (one per item) |
| `transfer_pause` | `id: TransferId` | `null` |
| `transfer_resume` | `id: TransferId` | `null` |
| `transfer_cancel` | `id: TransferId` | `null` |
| `transfer_retry` | `id: TransferId` | `null` |
| `transfer_conflict_reply` | `pending_id: ConflictId, choice: ConflictChoice, apply_to_all: bool` | `null` |
| `transfers_subscribe` | `on_event: Channel<TransferEvent>` (replaces previous subscriber for the calling window) | `null` |

`Endpoint = {kind:"local"} | {kind:"remote", sftp_id}`. `ConflictPolicy = "Ask"|"Overwrite"|"Skip"|"Rename"|"ResumeIfSmaller"`, `ConflictChoice = "Overwrite"|"Skip"|"Rename"|"Resume"`.
`TransferEvent` (tag `type`): `Queued{id, src_path, dst_path}`, `Started{id, total_bytes, files}`, `Progress{id, bytes_done, bytes_per_s}` (≤ 4 Hz),
`Conflict{id, pending_id, info: ConflictInfo}`, `FileDone{id, path, skipped}`, `Done{id}`, `Failed{id, message}`, `Paused{id}`, `Cancelled{id}`.

`file_open` opens a regular document or media file via the default OS application. It checks local roots after resolving links, or reads a temporary copy from an active SFTP session (≤ 256 MiB). Unsupported formats and oversized remote files return `Unsupported`.

`file_preview` reads one to four 5 MiB blocks of a regular file from its beginning or end. It checks resolved local roots or the active SFTP session and returns `Unsupported` for binary data or unsupported encodings. Preview content is rendered as plain text.

### crates/sftp public API (`crates/sftp/src/lib.rs`)
`Sftp::open(&Connection)`, `list/stat/mkdir/rename/remove(recursive)/chmod/realpath/close`, `copy_document(path, dst, max_bytes)`, `read_document(path, limit, tail)` (POSIX remote paths);
`local::{home, list, mkdir, rename, remove}`; `sanitize::{sanitize_name, safe_join}`;
`TransferEngine::new(TransferConfig{parallel_files: 3, pipeline_depth: 64, chunk_size: 32 KiB})` with `subscribe() -> mpsc::UnboundedReceiver<TransferEvent>`, `enqueue(TransferJob) -> Vec<TransferId>`, `pause/resume/cancel/retry(id)`, `conflict_reply(ConflictId, ConflictChoice, apply_to_all)`.
`TransferJob { src: Endpoint::{Local, Remote(Arc<Sftp>)}, dst, items, dst_dir, conflict }` (engine-side Endpoint, mapped by the bridge).

## Commands — stats (ADR-009, ADR-010)
| command | args | returns |
|---|---|---|
| `host_probe` | `host_id` | `HostReachability { reachable: bool, latency_ms: u32|null, checked_at: i64 }` — TCP reachability of resolved SSH port (3 s), no authentication. Unlocked vault required, passive; jump hosts → `Unsupported`. Errors: `VaultLocked`, `NotFound`, `Unsupported`, `Internal`. ADR-010. |
| `host_lookup_ip` | `host_id` | `string[]` — IP addresses of the host name (IPv4 first, ≤ 4), DNS only, 3 s, no connection. Unlocked vault required, passive. For the map's opt-in location lookup. Errors: `VaultLocked`, `NotFound`, `Timeout`, `Network`. ADR-011. |
| `stats_sample` | `session_id` | `HostStats` (~1 s: samples `/proc` twice, 1 s apart, over an exec channel of the session's connection). Errors: `NotFound`, `Network` (not connected), `Timeout` (8 s), `Unsupported` (non-Linux). Passive: does not reset auto-lock. |

`HostStats` (percent 0–100, bytes, bytes/s; u64 → TS `number`): `cpu_pct, cores, cpu_cores: number[], cpu_core_split: CoreSplit[]` (same order; `{ user, system, nice, iowait, steal }` %, ADR-012)`, cpu_user, cpu_system, cpu_nice, cpu_iowait, cpu_steal, load1, load5, load15, mem_total, mem_used (total − available), mem_cached, swap_total, swap_used, disk_total, disk_used (root fs), mem_free, net_rx_bps, net_tx_bps (non-lo), ifaces: NetIface[] (≤ 8, busiest first: name, rx_bps, tx_bps, rx_total, tx_total), disk_read_bps, disk_write_bps (whole disks), uptime_s, os_name: string|null, cpu_model: string|null, disk_dev_bytes (physical disk holding `/`), disk_kind: `NVMe`|`SSD`|`HDD`|null (ADR-015), temp_c: number|null, procs: ProcRow[]` (top 8 by CPU), `listen: Listener[]` (≤ 32: proto, addr, port, process|null), `flows: PeerFlow[]` (≤ 40, busiest first: inbound, port, peer, conns, bytes_in, bytes_out — lifetime bytes of the open TCP connections), `conns: u32`, `full: bool`. Slow fields (`disk_total/used`, `os_name`, `cpu_model`, `disk_dev_bytes`, `disk_kind`, `cores`, `procs`, `listen`, `flows`, `conns`) are re-read every 15 s; in between the backend copies them from the session's last full sample (ADR-009 amendment 2026-11-07).
`ProcRow { pid, user, cpu, mem_bytes, command }`. Server strings are stripped of control chars, ≤ 128 chars.

## Commands — services (ADR-012)
| command | args | returns |
|---|---|---|
| `docker_list` | `session_id` | `DockerReport { access: "Ok"\|"Missing"\|"Denied"\|"Down", containers: Container[] }` (≤ 150, running first; `Container { id (12 hex), name, image, state, status, ports, cpu_pct, mem_bytes, mem_limit }`, usage only for running ones). One `docker ps -a` + one `docker stats --no-stream` over an exec channel, 15 s. Read-only, passive. Errors: `NotFound`, `Network`, `Timeout`. |
| `docker_logs` | `session_id, container_id: string, lines: u32` | `string` — last 20–2000 lines with timestamps, stdout+stderr; colour codes (SGR `ESC[…m`, digits and `;` only) kept for display, every other escape / control char stripped, ≤ 512 KiB, 12 s. Read-only. Unlocked vault required. Errors: `VaultLocked`, `NotFound`, `Network`, `Timeout`, `Validation` (bad id / CLI error line). |
| `docker_action` | `session_id, container_id: string, action: "Start"\|"Stop"\|"Restart"` | `null` — the only state-changing command of this section; the UI confirms Stop / Restart first. Id must be 12–64 hex chars (passed to the CLI as an argument, never interpolated). 70 s. Unlocked vault required. Errors: as `docker_logs`. |
| `docker_compose` | `session_id` | `DockerCompose { access, projects: ComposeProject[] }` (≤ 150, busiest first; `ComposeProject { name, dir: string\|null, running, total, services: ComposeService[] }`, `ComposeService { service, container, state, status }`). Built from the labels `com.docker.compose.*` on `docker ps -a`; the compose plugin is not needed. Read-only, passive, 15 s. ADR-016. Errors: `NotFound`, `Network`, `Timeout`. |
| `docker_images` | `session_id` | `DockerImages { access, images: DockerImage[] }` (≤ 150, used first then biggest; `DockerImage { id (12 hex), name: "repo:tag", size_bytes: number\|null, created: string, containers: u32 }`). `docker images` + `docker ps -a`. Read-only, passive, 15 s. ADR-016. Errors: as `docker_compose`. |
| `docker_volumes` | `session_id` | `DockerVolumes { access, volumes: DockerVolume[] }` (≤ 150; `DockerVolume { name, driver, size_bytes: number\|null, containers: u32 }`). `docker volume ls`, mounts of `docker ps -a`, size from `docker system df -v`. Read-only, passive, 25 s. ADR-016. Errors: as `docker_compose`. |
| `docker_networks` | `session_id` | `DockerNetworks { access, networks: DockerNetwork[] }` (≤ 100; `DockerNetwork { id (12 hex), name, driver, scope, subnet: string, containers: u32 }`). `docker network ls` + `inspect`. Read-only, passive, 15 s. ADR-016. Errors: as `docker_compose`. |
| `docker_events` | `session_id, window_s: u32` | `DockerEvents { access, events: DockerEvent[] }` (≤ 200, newest first; `DockerEvent { time: i64 (Unix ms, server clock), kind: "container"\|"image"\|"volume"\|"network", action, name }`). `window_s` is clamped to 10–86 400. `docker events --since … --until now` returns by itself, so no stream stays open; exec / health / attach noise and command lines after `:` are dropped. Read-only, passive, 15 s. ADR-016. Errors: as `docker_compose`. |
| `cert_list` | `session_id` | `CertReport { certs: CertInfo[], denied: u32, no_openssl: bool }`, `CertInfo { path, names: string[] (CN first, ≤ 20), issuer: string\|null, not_after: i64 (Unix ms) }`, soonest first, duplicates merged. Reads only certificate files of Let's Encrypt `live`, nginx/Apache `ssl_certificate`/`SSLCertificateFile`, Caddy, `/root/cert` via `openssl x509` (public fields only), ≤ 48 files, 15 s. Read-only, passive. Errors: `NotFound`, `Network`, `Timeout`. |

## Commands — app
| command | args | returns |
|---|---|---|
| `tray_set` | `menu: TrayMenu\|null` | `null` — shows/updates the menu-bar icon (`TrayMenu { tooltip, items: TrayItem[] (≤ 24), open_label, quit_label }`, `TrayItem { host_id: HostId\|null, text }`); `null` removes it. Strings are rendered as native menu text, control chars stripped, ≤ 120 chars. Vault lock drops the server lines on the backend side. Passive. ADR-013. |
| `app_info` | — | `AppInfo { version, platform, arch }` |
| `open_external` | `url: string` (http/https only, validated) | `null` |
| `app_notify` | `title: string, body: string` (desktop: system notification shown by the backend; control chars dropped, ≤ 120 / 300 chars; ADR-017) | `null` (`Unsupported` on mobile or when the OS refuses) |

## Global events (`app.emit`, typed via tauri-specta `Event`)
| event name | Rust type | payload | TS (`apiEvents.listen(...)`) |
|---|---|---|---|
| `vault://locked` | `VaultLocked` | `null` | `vaultLocked` |
| `tree://changed` | `TreeChanged` | `null` | `treeChanged` |
| `update://available` | `UpdateAvailable` | `{ version, notes: string\|null }` | `updateAvailable` |
| `tray://open-host` | `TrayOpenHost` | `{ host_id }` | `trayOpenHost` |

## Rules
- No command returns secret material. Ever.
- Paths from UI are validated: remote paths are POSIX strings; local paths must come from the dialog plugin, `local_home`, or be under a path previously returned by `local_list`.
- Every command has a mock in `ui/src/lib/api/mock.ts` (key of `mockApi: Api`, type-checked against the bindings); `node scripts/check-mock-coverage.mjs` fails on any command in this file without a mock (run by `just bindings`).
- `bindings_are_fresh` test in `src-tauri` fails when `bindings.ts` is stale.
