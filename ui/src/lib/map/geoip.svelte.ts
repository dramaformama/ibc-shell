/**
 * Opt-in server location by IP (ADR-011): host name → IP on the backend (`hostLookupIp`), then
 * city and coordinates from ipapi.is (ipinfo.io when that has no answer). Each answer is kept for good (settings `map.geo`) and is
 * looked up again only when the host's address is edited; deleting the host drops it. Only
 * public server IPs are sent; nothing happens until the user turns it on.
 */
import { api, type HostId, type Value } from '$lib/api';
import { geoFetch } from '$lib/api/mock/geo';

const ON_KEY = 'map.geoip';
const CACHE_KEY = 'map.geo';
const ENDPOINT = 'https://api.ipapi.is/';
const FALLBACK = 'https://ipinfo.io/';
const PARALLEL = 4;

export interface GeoHit {
  /** The host's address (name or IP) the answer belongs to; a different one means re-lookup. */
  addr: string;
  ip: string;
  /** Lowercase ISO country code. */
  cc: string;
  city: string;
  lat: number;
  lon: number;
  /** Unix ms of the lookup. */
  at: number;
}

const fetcher: typeof fetch = import.meta.env.VITE_API === 'real' ? fetch : geoFetch;

/** Private, loopback, link-local, CGNAT, documentation and other non-routable ranges. */
export function isPublicIp(ip: string): boolean {
  if (ip.includes(':')) {
    const v6 = ip.toLowerCase();
    return !(
      v6 === '::1' ||
      v6 === '::' ||
      /^f[cd]/.test(v6) ||
      /^fe[89ab]/.test(v6) ||
      v6.startsWith('2001:db8') ||
      v6.startsWith('::ffff:')
    );
  }
  const p = ip.split('.').map(Number);
  if (p.length !== 4 || p.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) return false;
  const [a, b, c] = p as [number, number, number, number];
  return !(
    a === 0 ||
    a === 10 ||
    a === 127 ||
    a >= 224 ||
    (a === 100 && b >= 64 && b <= 127) ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b <= 31) ||
    (a === 192 && b === 168) ||
    (a === 192 && b === 0 && (c === 0 || c === 2)) ||
    (a === 198 && (b === 18 || b === 19)) ||
    (a === 198 && b === 51 && c === 100) ||
    (a === 203 && b === 0 && c === 113)
  );
}

const CC = /^[a-z]{2}$/;

/**
 * Reads an ipapi.is answer, either the documented `location` block or the flat form
 * (`lat`/`lon`/`city`) the free tier sometimes returns; null without usable coordinates.
 */
export function parseAnswer(addr: string, ip: string, body: unknown, now: number): GeoHit | null {
  const b = body as Record<string, unknown> | null;
  if (!b || typeof b !== 'object') return null;
  const loc = (b.location && typeof b.location === 'object' ? b.location : b) as Record<
    string,
    unknown
  >;
  const lat = Number(loc.latitude ?? loc.lat);
  const lon = Number(loc.longitude ?? loc.lon);
  const code = loc.country_code ?? loc.countryCode ?? loc.country;
  const cc = typeof code === 'string' ? code.toLowerCase() : '';
  if (!Number.isFinite(lat) || !Number.isFinite(lon) || !CC.test(cc)) return null;
  const city = typeof loc.city === 'string' ? loc.city.slice(0, 64) : '';
  return { addr, ip, cc, city, lat, lon, at: now };
}

/** Reads an ipinfo.io answer (`loc` is `"lat,lon"`); the second source when ipapi.is fails. */
export function parseIpinfo(addr: string, ip: string, body: unknown, now: number): GeoHit | null {
  const b = body as { loc?: unknown; country?: unknown; city?: unknown } | null;
  if (!b || typeof b.loc !== 'string') return null;
  const [lat, lon] = b.loc.split(',').map(Number) as [number, number];
  const cc = typeof b.country === 'string' ? b.country.toLowerCase() : '';
  if (!Number.isFinite(lat) || !Number.isFinite(lon) || !CC.test(cc)) return null;
  const city = typeof b.city === 'string' ? b.city.slice(0, 64) : '';
  return { addr, ip, cc, city, lat, lon, at: now };
}

class GeoStore {
  enabled = $state(false);
  hits = $state.raw<Record<string, GeoHit>>({});
  busy = $state(0);
  #tried = new Set<string>();

  async load(): Promise<void> {
    const [on, cache] = await Promise.all([api.settingsGet(ON_KEY), api.settingsGet(CACHE_KEY)]);
    this.enabled = on.status === 'ok' && on.data === true;
    if (cache.status === 'ok' && cache.data && typeof cache.data === 'object') {
      const out: Record<string, GeoHit> = {};
      for (const [k, v] of Object.entries(cache.data as Record<string, unknown>)) {
        const h = v as Partial<GeoHit> | null;
        if (
          h &&
          typeof h.addr === 'string' &&
          typeof h.ip === 'string' &&
          typeof h.cc === 'string' &&
          typeof h.city === 'string' &&
          typeof h.at === 'number' &&
          Number.isFinite(h.lat) &&
          Number.isFinite(h.lon)
        )
          out[k] = h as GeoHit;
      }
      this.hits = out;
    }
  }

  setEnabled(on: boolean): void {
    this.enabled = on;
    void api.settingsSet(ON_KEY, on);
  }

  get(hostId: HostId | null | undefined): GeoHit | null {
    return this.enabled && hostId ? (this.hits[hostId] ?? null) : null;
  }

  /**
   * Looks up only hosts that have no answer for their current address (new or edited); a
   * failed host is retried next session. Answers of deleted hosts are dropped.
   */
  async locate(hosts: { id: HostId; hostname: string }[]): Promise<void> {
    if (!this.enabled) return;
    const ids = new Set(hosts.map((h) => h.id));
    const kept = Object.fromEntries(Object.entries(this.hits).filter(([id]) => ids.has(id)));
    if (Object.keys(kept).length !== Object.keys(this.hits).length) {
      this.hits = kept;
      this.#save();
    }
    const todo = hosts.filter((h) => {
      const key = `${h.id}|${h.hostname}`;
      if (this.hits[h.id]?.addr === h.hostname || this.#tried.has(key)) return false;
      this.#tried.add(key);
      return true;
    });
    let next = 0;
    const worker = async () => {
      while (next < todo.length && this.enabled) await this.#one(todo[next++]!);
    };
    await Promise.all(Array.from({ length: Math.min(PARALLEL, todo.length) }, worker));
  }

  async #one({ id, hostname }: { id: HostId; hostname: string }): Promise<void> {
    this.busy++;
    try {
      const r = await api.hostLookupIp(id);
      const ip = r.status === 'ok' ? r.data.find(isPublicIp) : undefined;
      if (!ip) return;
      const hit = await this.#ask(hostname, ip);
      if (!hit) return;
      this.hits = { ...this.hits, [id]: hit };
      this.#save();
    } catch {
      /* offline or rate-limited: keep the flag position */
    } finally {
      this.busy--;
    }
  }

  /** ipapi.is first, ipinfo.io when it has no usable answer (limit, outage, odd format). */
  async #ask(addr: string, ip: string): Promise<GeoHit | null> {
    const get = async (url: string): Promise<unknown> => {
      const res = await fetcher(url, {
        credentials: 'omit',
        referrerPolicy: 'no-referrer',
        signal: AbortSignal.timeout(8000)
      });
      return res.ok ? res.json() : null;
    };
    const first = await get(`${ENDPOINT}?q=${encodeURIComponent(ip)}`).catch(() => null);
    const hit = parseAnswer(addr, ip, first, Date.now());
    if (hit) return hit;
    const second = await get(`${FALLBACK}${encodeURIComponent(ip)}/json`).catch(() => null);
    return parseIpinfo(addr, ip, second, Date.now());
  }

  #save(): void {
    void api.settingsSet(CACHE_KEY, this.hits as unknown as Value);
  }

  clear(): void {
    this.enabled = false;
    this.hits = {};
    this.#tried.clear();
  }
}

export const geo = new GeoStore();
