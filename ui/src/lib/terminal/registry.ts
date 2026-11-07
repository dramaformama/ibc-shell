/**
 * Live terminals by tab id, so snippets can type into a shell without owning it. Each
 * TerminalView registers itself while mounted.
 */
import type { Phase } from './connection.svelte';

export interface TermHandle {
  /** Types text into the shell as if the user had (no paste confirmation, no echo here). */
  send(text: string): void;
  phase(): Phase;
  focus(): void;
}

const live = new Map<string, TermHandle>();

export function registerTerminal(tabId: string, h: TermHandle): () => void {
  live.set(tabId, h);
  return () => {
    if (live.get(tabId) === h) live.delete(tabId);
  };
}

export function terminal(tabId: string): TermHandle | null {
  return live.get(tabId) ?? null;
}

/**
 * Resolves with the handle once the tab's shell is connected; null after `timeoutMs` or if it
 * fails / disconnects first (a host-key or password prompt simply keeps it waiting).
 */
export async function whenConnected(tabId: string, timeoutMs = 60_000): Promise<TermHandle | null> {
  const until = Date.now() + timeoutMs;
  for (;;) {
    const h = live.get(tabId);
    const p = h?.phase();
    if (h && p === 'connected') return h;
    if (p === 'failed' || p === 'disconnected' || Date.now() > until) return null;
    await new Promise((r) => setTimeout(r, 200));
  }
}
