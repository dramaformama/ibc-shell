/** One wording for a server's state everywhere (tile, card, map). */
import * as m from '$lib/paraglide/messages.js';
import type { Server } from '$lib/fleet/model';

export type StatusTone = 'connected' | 'connecting' | 'failed' | 'online' | 'offline' | 'unknown';

export function serverStatus(s: Server, checking: boolean): { tone: StatusTone; text: string } {
  if (s.phase === 'connected') return { tone: 'connected', text: m.status_connected() };
  if (s.phase === 'connecting') return { tone: 'connecting', text: m.dash_connecting() };
  if (s.phase === 'failed') return { tone: 'failed', text: m.dash_connection_error() };
  if (s.availability === 'online') return { tone: 'online', text: m.dash_available() };
  if (s.availability === 'offline') return { tone: 'offline', text: m.dash_unavailable() };
  return { tone: 'unknown', text: checking ? m.dash_checking() : m.dash_not_checked() };
}

/** Worst-first rank for picking one colour for several servers (map pins). */
export const toneRank: Record<StatusTone, number> = {
  failed: 0,
  offline: 1,
  connecting: 2,
  connected: 3,
  online: 4,
  unknown: 5
};
