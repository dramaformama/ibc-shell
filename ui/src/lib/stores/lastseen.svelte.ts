/**
 * Last known metrics per saved host, kept across restarts (settings `stats.last`), so the
 * overview and the Status page show "what it was" before a connection is up. Connection lists
 * (`flows`, `listen`) are not stored: remote peers' addresses stay in memory only.
 *
 * Also the "watch" list (settings `stats.watch`): hosts connected in the background after
 * unlock, so their metrics are live from the start.
 */
import { api, type HostId, type Value } from '$lib/api';
import type { Series } from './stats.svelte';

const KEY = 'stats.last';
const WATCH_KEY = 'stats.watch';
/** History points kept per host (≈ 2.5 min at one point per 5 s). */
const KEEP = 30;
/** A host's snapshot is written at most this often. */
const SAVE_EVERY_MS = 60_000;

type Arr = (number | null)[];
export interface Snapshot {
  at: number;
  d: NonNullable<Series['latest']>;
  cpu: Arr;
  mem: Arr;
  load1: Arr;
  load5: Arr;
  load15: Arr;
  rx: Arr;
  tx: Arr;
  dr: Arr;
  dw: Arr;
}
const SERIES = ['cpu', 'mem', 'load1', 'load5', 'load15', 'rx', 'tx', 'dr', 'dw'] as const;

const isSnapshot = (v: unknown): v is Snapshot =>
  !!v &&
  typeof v === 'object' &&
  typeof (v as Snapshot).at === 'number' &&
  !!(v as Snapshot).d &&
  typeof (v as Snapshot).d === 'object' &&
  SERIES.every((k) => Array.isArray((v as Snapshot)[k]));

class LastSeenStore {
  items = $state.raw<Record<string, Snapshot>>({});
  watched = $state.raw<HostId[]>([]);
  #savedAt = new Map<string, number>();
  #timer: ReturnType<typeof setTimeout> | null = null;

  async load(): Promise<void> {
    const [last, watch] = await Promise.all([api.settingsGet(KEY), api.settingsGet(WATCH_KEY)]);
    if (last.status === 'ok' && last.data && typeof last.data === 'object') {
      const out: Record<string, Snapshot> = {};
      for (const [k, v] of Object.entries(last.data as Record<string, unknown>))
        if (isSnapshot(v)) out[k] = { ...v, d: { ...v.d, flows: [], listen: [] } };
      this.items = out;
    }
    if (watch.status === 'ok' && Array.isArray(watch.data))
      this.watched = watch.data.filter((v): v is string => typeof v === 'string');
  }

  get(hostId: HostId | null | undefined): Snapshot | null {
    return hostId ? (this.items[hostId] ?? null) : null;
  }

  /** Remembers a live series; persisted at most once a minute per host. */
  record(hostId: HostId, s: Series): void {
    if (!s.latest || !s.updatedAt) return;
    const now = Date.now();
    if (now - (this.#savedAt.get(hostId) ?? 0) < SAVE_EVERY_MS) return;
    this.#savedAt.set(hostId, now);
    const tail = (k: (typeof SERIES)[number]) => s[k].slice(-KEEP);
    const snap: Snapshot = {
      at: s.updatedAt,
      d: { ...s.latest, flows: [], listen: [], conns: 0 },
      cpu: tail('cpu'),
      mem: tail('mem'),
      load1: tail('load1'),
      load5: tail('load5'),
      load15: tail('load15'),
      rx: tail('rx'),
      tx: tail('tx'),
      dr: tail('dr'),
      dw: tail('dw')
    };
    this.items = { ...this.items, [hostId]: snap };
    this.#persist();
  }

  /** A series to draw from a snapshot: marked stale, no connection data. */
  toSeries(snap: Snapshot): Series {
    return {
      latest: snap.d,
      updatedAt: snap.at,
      stale: true,
      unsupported: false,
      flowRates: {},
      flowBase: null,
      ...Object.fromEntries(SERIES.map((k) => [k, snap[k]]))
    } as Series;
  }

  isWatched(hostId: HostId): boolean {
    return this.watched.includes(hostId);
  }

  setWatched(hostId: HostId, on: boolean): void {
    this.watched = on
      ? [...this.watched.filter((id) => id !== hostId), hostId]
      : this.watched.filter((id) => id !== hostId);
    void api.settingsSet(WATCH_KEY, this.watched);
  }

  /** Lock: drop the in-memory copy (the settings file keeps it). */
  clear(): void {
    this.items = {};
    this.watched = [];
    this.#savedAt.clear();
  }

  #persist(): void {
    if (this.#timer) return;
    this.#timer = setTimeout(() => {
      this.#timer = null;
      void api.settingsSet(KEY, this.items as unknown as Value);
    }, 2000);
  }
}

export const lastSeen = new LastSeenStore();
