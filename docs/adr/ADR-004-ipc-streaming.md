# ADR-004: Raw-bytes IPC streaming for terminals
Date: 2026-11-07 · Status: Accepted

## Context
Terminal output can reach tens of MB/s. Verified in tauri 2.12.5 (`src/ipc/channel.rs` docs):
`Channel<Vec<u8>>` serializes as a JSON number array (~4× size + parse cost); only
`InvokeResponseBody::Raw` (default `Channel<InvokeResponseBody>`) or `ipc::Response` arrive in JS as
`ArrayBuffer`. tauri-specta cannot type `InvokeResponseBody` or a raw `Request` body.

## Decision
Rust → UI: `Channel<RawBytes>`; `RawBytes(Vec<u8>)` (src-tauri/src/api/stream.rs) is NOT `Serialize`,
implements `IpcResponse` → `InvokeResponseBody::Raw`, and `specta::Type` → TS `ArrayBuffer`.
Coalesce 64 KiB / 8 ms; credit window 1 MiB acked via `shell_ack`.
UI → Rust: `shell_write` takes `tauri::ipc::Request` (raw body + header `x-session-id`), is excluded
from the specta builder (`api::RAW_COMMANDS`), registered via a combined invoke handler, and called
through hand-written `ui/src/lib/api/raw.ts` (`api.shellWrite`). `i64`/`u64` are exported as TS
`number` (`dangerously_cast_bigints_to_number`): millis and byte sizes stay < 2^53.

## Consequences
One hand-written TS wrapper to keep in sync (covered by the `Api` type and mock coverage check).
Any new binary stream must use `RawBytes`; `Channel<Vec<u8>>` is forbidden. Contract: ipc.md v1.
