/**
 * Live server metrics (ADR-009): polls `statsSample` over an open terminal session while some
 * view (dashboard card, inspector) is watching, and keeps a short history for the charts.
 */
import { api, type HostStats, type Result, type SessionId } from '$lib/api';
import { hidden, whenVisible } from '$lib/util/visible';

const HISTORY = 60;
/** Pause after each answer; one sample takes ~1 s, so a point every ~5 s (ADR-009 amendment). */
const EVERY_MS = 4000;

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
  /** Disk read / write, bytes/s. */
  dr: (number | null)[];
  dw: (number | null)[];
  /** The server cannot report metrics (not Linux, no /proc). */
  unsupported: boolean;
  /** Per-flow rates (bytes/s in, out) from the last two full samples, keyed by `flowKey`. */
  flowRates: Record<string, [number, number]>;
  /** Last full sample's flow byte counters and its time, for the next difference. */
  flowBase: { at: number; bytes: Record<string, [number, number]> } | null;
}

export const flowKey = (f: { inbound: boolean; port: number; peer: string }) =>
  `${f.inbound ? 'in' : 'out'}|${f.port}|${f.peer}`;

/** Rates between two full samples; closed connections make a counter drop, which reads as 0. */
function flowRates(s: Series, d: HostStats, now: number): Pick<Series, 'flowRates' | 'flowBase'> {
  if (!d.full) return { flowRates: s.flowRates, flowBase: s.flowBase };
  const bytes: Record<string, [number, number]> = {};
  for (const f of d.flows) bytes[flowKey(f)] = [f.bytes_in, f.bytes_out];
  const rates: Record<string, [number, number]> = {};
  const prev = s.flowBase;
  const dt = prev ? (now - prev.at) / 1000 : 0;
  if (prev && dt > 1) {
    for (const [k, [i, o]] of Object.entries(bytes)) {
      const p = prev.bytes[k];
      if (p) rates[k] = [Math.max(0, i - p[0]) / dt, Math.max(0, o - p[1]) / dt];
    }
  }
  return { flowRates: rates, flowBase: { at: now, bytes } };
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
  dr: [],
  dw: [],
  unsupported: false,
  flowRates: {},
  flowBase: null
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
      if (hidden()) {
        await whenVisible();
        if (!this.#watchers.has(sid)) break;
      }
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
        const now = Date.now();
        this.data[sid] = {
          ...flowRates(s, d, now),
          latest: d,
          updatedAt: now,
          stale: false,
          cpu: push(s.cpu, d.cpu_pct),
          mem: push(s.mem, d.mem_total ? (d.mem_used / d.mem_total) * 100 : 0),
          load1: push(s.load1, d.load1),
          load5: push(s.load5, d.load5),
          load15: push(s.load15, d.load15),
          rx: push(s.rx, d.net_rx_bps),
          tx: push(s.tx, d.net_tx_bps),
          dr: push(s.dr, d.disk_read_bps),
          dw: push(s.dw, d.disk_write_bps),
          unsupported: false
        };
      } else {
        failures++;
        this.data[sid] = { ...s, stale: true };
        if (r.error.code === 'Unsupported')
          this.data[sid] = { ...s, stale: true, unsupported: true };
        // A closed session or a server that cannot report: stop quietly.
        if (r.error.code === 'NotFound') delete this.data[sid];
        if (r.error.code === 'Unsupported' || r.error.code === 'NotFound' || failures >= 5) break;
      }
      await new Promise((ok) => setTimeout(ok, failures ? EVERY_MS * 4 : EVERY_MS));
    }
  }

  forget(sid: SessionId): void {
    delete this.data[sid];
  }

  /** Lock: drop every sample (the loops stop with their watchers). */
  clear(): void {
    this.#watchers.clear();
    this.data = {};
  }
}

export const stats = new StatsStore();
