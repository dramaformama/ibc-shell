/**
 * Live server metrics (ADR-009): polls `statsSample` over an open terminal session while some
 * view (dashboard card, inspector) is watching, and keeps a short history for the charts.
 */
import { api, type HostStats, type Result, type SessionId } from '$lib/api';

const HISTORY = 60;
const EVERY_MS = 2000;

export interface Series {
  latest: HostStats | null;
  updatedAt: number | null;
  stale: boolean;
  cpu: (number | null)[];
  mem: (number | null)[];
  load1: (number | null)[];
  load5: (number | null)[];
  load15: (number | null)[];
  rx: (number | null)[];
  tx: (number | null)[];
  /** The server cannot report metrics (not Linux, no /proc). */
  unsupported: boolean;
}

const empty = (): Series => ({
  latest: null,
  updatedAt: null,
  stale: false,
  cpu: [],
  mem: [],
  load1: [],
  load5: [],
  load15: [],
  rx: [],
  tx: [],
  unsupported: false
});
const push = (a: (number | null)[], v: number | null) => [...a.slice(-(HISTORY - 1)), v];

class StatsStore {
  /** Keyed by session id. */
  data = $state<Record<string, Series>>({});
  #watchers = new Map<string, number>();
  #running = new Set<string>();

  get(sid: SessionId | null): Series | null {
    return sid ? (this.data[sid] ?? null) : null;
  }

  /** Starts (or joins) polling for a session; returns the stop function. */
  watch(sid: SessionId): () => void {
    const n = this.#watchers.get(sid) ?? 0;
    this.#watchers.set(sid, n + 1);
    if (!this.#running.has(sid)) {
      this.#running.add(sid);
      void this.#loop(sid).finally(() => this.#running.delete(sid));
    }
    let stopped = false;
    return () => {
      if (stopped) return;
      stopped = true;
      const left = (this.#watchers.get(sid) ?? 1) - 1;
      if (left <= 0) this.#watchers.delete(sid);
      else this.#watchers.set(sid, left);
    };
  }

  async #loop(sid: SessionId): Promise<void> {
    this.data[sid] ??= empty();
    let failures = 0;
    while (this.#watchers.has(sid)) {
      let r: Result<HostStats>;
      try {
        r = await api.statsSample(sid);
      } catch {
        r = { status: 'error', error: { code: 'Network', detail: 'metrics unavailable' } };
      }
      if (!this.#watchers.has(sid)) break;
      const s: Series = this.data[sid] ?? empty();
      if (r.status === 'ok') {
        failures = 0;
        const d = r.data;
        this.data[sid] = {
          latest: d,
          updatedAt: Date.now(),
          stale: false,
          cpu: push(s.cpu, d.cpu_pct),
          mem: push(s.mem, d.mem_total ? (d.mem_used / d.mem_total) * 100 : 0),
          load1: push(s.load1, d.load1),
          load5: push(s.load5, d.load5),
          load15: push(s.load15, d.load15),
          rx: push(s.rx, d.net_rx_bps),
          tx: push(s.tx, d.net_tx_bps),
          unsupported: false
        };
      } else {
        failures++;
        this.data[sid] = { ...s, stale: true };
        if (r.error.code === 'Unsupported')
          this.data[sid] = { ...s, stale: true, unsupported: true };
        // A closed session or a server that cannot report: stop quietly.
        if (r.error.code === 'Unsupported' || r.error.code === 'NotFound' || failures >= 5) break;
      }
      await new Promise((ok) => setTimeout(ok, failures ? EVERY_MS * 4 : EVERY_MS));
    }
    if (!this.#watchers.has(sid)) delete this.data[sid];
  }

  forget(sid: SessionId): void {
    delete this.data[sid];
  }
}

export const stats = new StatsStore();
