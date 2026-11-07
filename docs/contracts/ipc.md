# Contract: IPC (src-tauri ↔ ui)
Status: DRAFT → `FROZEN v1` after Phase 0. Commands are `#[tauri::command] #[specta::specta] async fn`, return `Result<T, ApiError>`.

## Errors
```rust
#[derive(Serialize, specta::Type)]
#[serde(tag = "code", content = "detail")]
pub enum ApiError {
  VaultLocked, WrongPassword, NotFound, Validation(String), Cycle, TooDeep,
  HostKeyUnknown(HostKeyInfo),      // UI shows TOFU dialog then calls hostkey_accept
  HostKeyChanged(HostKeyMismatch),  // UI shows red block, NO accept button in v1 (only "remove old key in Known hosts")
  AuthFailed { methods_left: Vec<String> },
  Network(String), Timeout, Sftp(String), Cancelled, Conflict(ConflictInfo), Internal(String) // message must not contain secrets
}
```

## Commands — store / tree
| command | args | returns |
|---|---|---|
| `tree_list` | — | `Tree { folders: FolderDto[], hosts: HostDto[] }` |
| `folder_create` | `FolderInput` | `FolderDto` |
| `folder_update` | `id, FolderPatch` | `FolderDto` |
| `folder_move` | `id, parent: Id \| null, after: NodeRef \| null` | `()` |
| `folder_delete` | `id, mode: "MoveToParent" \| "DeleteContents"` | `()` |
| `host_create` / `host_update` / `host_duplicate` / `host_delete` / `host_move` | see data-model | `HostDto` / `()` |
| `host_resolve` | `id` | `ResolvedHostDto` (each field `{value, source}`) |
| `host_search` | `q, limit` | `Id[]` |
| `identity_list` / `identity_create` / `identity_update` / `identity_delete` | `IdentityInput` contains `password?: string`, `private_key_pem?: string`, `passphrase?: string` — accepted ONCE, moved into `SecretString`, never returned | `IdentityDto` (no secrets: label, kind, key_algo, fingerprint, public_key) |
| `identity_import_file` | `path` (from dialog plugin), `passphrase?` | `IdentityDto` |
| `tag_*`, `snippet_*`, `forward_*`, `known_host_list` / `known_host_delete` | CRUD | DTOs |
| `settings_get` / `settings_set` | `key, value: JSON` | JSON |

## Commands — vault
| `vault_status` | — | `{ state: "Uninitialized" \| "Locked" \| "Unlocked", mode: "OsKeychain" \| "MasterPassword", biometric_available: bool }` |
| `vault_init` | `mode, password?` | `()` |
| `vault_unlock` | `password?` (absent = keychain/biometric) | `()` |
| `vault_lock` | — | `()` |
| `vault_change_password` | `old, new` | `()` |
| `vault_export` / `vault_import` | `path, password` | `()` / `ImportSummary` |

## Commands — sessions (SSH)
| `session_open` | `host_id, cols, rows, on_data: Channel<Vec<u8>>, on_event: Channel<SessionEvent>` | `SessionId` |
| `session_open_adhoc` | `QuickConnect { user, hostname, port }` + channels | `SessionId` |
| `shell_write` | `session_id, data: Vec<u8>` — sent as `tauri::ipc::Request` raw body (InvokeBody::Raw), header `x-session-id` | `()` |
| `shell_ack` | `session_id, bytes: u32` | `()` |
| `shell_resize` | `session_id, cols, rows` | `()` |
| `session_close` | `session_id` | `()` |
| `session_reconnect` | `session_id` | `()` |
| `hostkey_accept` | `pending_id, remember: bool` | `()` |
| `kbd_interactive_reply` | `pending_id, answers: string[]` | `()` |
| `passphrase_reply` | `pending_id, passphrase: string \| null` | `()` |

`SessionEvent` (tagged enum): `Connecting{step}`, `HostKeyPrompt{pending_id, info}`, `KbdInteractive{pending_id, name, instruction, prompts:[{text, echo}]}`, `PassphrasePrompt{pending_id, identity_label}`, `Connected{server_banner?}`, `Disconnected{reason, can_reconnect}`, `ExitStatus{code}`, `ForwardStatus{forward_id, state, error?}`.

### Streaming & backpressure (normative)
- Rust → UI: `on_data` carries raw `Vec<u8>` (Channel serializes as ArrayBuffer — verify on Tauri version; if not, send via `tauri::ipc::Response::new(bytes)` pattern per ADR-004). Coalesce: flush when buffer ≥ 64 KiB or 8 ms elapsed.
- Credit window: Rust stops reading from the SSH channel when unacked bytes > 1 MiB. UI calls `shell_ack(bytes)` from xterm `write(data, callback)` completion.
- UI → Rust: keystrokes batched per animation frame.

## Commands — SFTP
| `sftp_open` | `session_id` (reuses SSH connection) or `host_id` | `SftpId` |
| `sftp_list` | `sftp_id, path` | `DirListing { path, entries: Entry[] }` Entry = `{name, kind: "file"\|"dir"\|"symlink"\|"other", size, mtime, mode, link_target?}` |
| `sftp_stat` / `sftp_mkdir` / `sftp_rename` / `sftp_remove` (recursive flag) / `sftp_chmod` / `sftp_realpath` | … | … |
| `local_list` / `local_mkdir` / `local_rename` / `local_remove` | same shapes for local FS (desktop only) | … |
| `transfer_enqueue` | `TransferRequest { src: Endpoint, dst: Endpoint, items: string[], conflict: ConflictPolicy }` Endpoint = `{kind:"local"} \| {kind:"remote", sftp_id}`; ConflictPolicy = `"Ask"\|"Overwrite"\|"Skip"\|"Rename"\|"ResumeIfSmaller"` | `TransferId[]` |
| `transfer_pause` / `transfer_resume` / `transfer_cancel` / `transfer_retry` | `id` | `()` |
| `transfer_conflict_reply` | `pending_id, choice, apply_to_all` | `()` |
| `transfers_subscribe` | `on_event: Channel<TransferEvent>` | `()` |

`TransferEvent`: `Queued`, `Started{id, total_bytes, files}`, `Progress{id, bytes_done, bytes_per_s}` (≤ 4 Hz per transfer), `Conflict{pending_id, info}`, `FileDone`, `Done{id}`, `Failed{id, error}`, `Paused`, `Cancelled`.

## Commands — app
| `app_info` | — | `{version, platform, arch}` |
| `open_external` | `url` (http/https only, validated) | `()` |

## Global events (`app.emit`)
`vault://locked`, `tree://changed` (after import), `update://available`.

## Rules
- No command returns secret material. Ever.
- Paths from UI are validated: remote paths are POSIX strings; local paths must come from dialog plugin or be under a path previously returned by `local_list`.
- Every command has a mock in `ui/src/lib/api/mock.ts` with identical signature.
