/** Real implementations of the raw (non-specta) commands and event adapter. ADR-004. */
import { invoke } from '@tauri-apps/api/core';
import { events } from './bindings';
import type { ApiError, ApiEvents, RawCommands, Result } from './types';

export const rawCommands: RawCommands = {
  async shellWrite(sessionId, data): Promise<Result<null>> {
    try {
      await invoke('shell_write', data, { headers: { 'x-session-id': sessionId } });
      return { status: 'ok', data: null };
    } catch (e) {
      if (e instanceof Error) throw e;
      return { status: 'error', error: e as ApiError };
    }
  }
};

export const realEvents: ApiEvents = {
  listen(name, cb) {
    // Each generated event has the same listen() shape; payload type is keyed by `name`.
    const ev = events[name] as unknown as {
      listen: (h: (e: { payload: unknown }) => void) => Promise<() => void>;
    };
    return ev.listen((e) => cb(e.payload as never));
  }
};
