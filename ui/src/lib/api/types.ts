/**
 * API surface shared by the real bridge (`bindings.ts` + `raw.ts`) and `mock.ts`.
 * Contract: docs/contracts/ipc.md (FROZEN v1). Types come from the generated bindings.
 */
import type { commands } from './bindings';
import type { ApiError, SessionId, TreeChanged, UpdateAvailable, VaultLocked } from './bindings';

export type * from './bindings';

/** Result shape of every command (tauri-specta `ErrorHandlingMode::Result`). */
export type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: ApiError };

/** Typed commands, exactly as generated. */
export type Commands = typeof commands;

/** Commands that bypass tauri-specta (raw request body, ADR-004). */
export interface RawCommands {
  /** Keystrokes → `shell_write` with raw body + `x-session-id` header. Batch per animation frame. */
  shellWrite(sessionId: SessionId, data: Uint8Array): Promise<Result<null>>;
}

/** Everything the UI may call. `mock.ts` and the real implementation both satisfy this. */
export type Api = Commands & RawCommands;

/** Global event payloads (`ipc.md` §Global events), keyed by binding accessor name. */
export interface EventPayloads {
  vaultLocked: VaultLocked;
  treeChanged: TreeChanged;
  updateAvailable: UpdateAvailable;
}

/** Global event subscription, identical for mock and real. Returns an unsubscribe function. */
export interface ApiEvents {
  listen<K extends keyof EventPayloads>(
    name: K,
    cb: (payload: EventPayloads[K]) => void
  ): Promise<() => void>;
}
