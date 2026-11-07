/**
 * Public API facade for UI code: `import { api, apiEvents } from '$lib/api'`.
 * `VITE_API=real` (set by `pnpm tauri dev`, see tauri-bridge P2) uses the Rust bridge;
 * anything else uses the in-memory mock.
 */
import { commands } from './bindings';
import { mockApi, mockEvents } from './mock';
import { rawCommands, realEvents } from './raw';
import type { Api, ApiEvents } from './types';

const useReal = import.meta.env.VITE_API === 'real';

export const api: Api = useReal ? { ...commands, ...rawCommands } : mockApi;
export const apiEvents: ApiEvents = useReal ? realEvents : mockEvents;

export type * from './types';
