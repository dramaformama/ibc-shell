/** Number formatting for the status cards (compact, Apple-style units). */

/** Unit labels; the app sets the localized ones at start (`setUnitLabels`). */
let UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];
let PER_SECOND = '/s';

/** Localized byte units (B, KB, MB, GB, TB) and the per-second suffix. */
export function setUnitLabels(units: string[], perSecond: string): void {
  if (units.length === 5) UNITS = units;
  PER_SECOND = perSecond;
}

/** 16.0 GB, 452 MB, 812 KB. */
export function bytes(n: number): string {
  let v = Math.max(0, n);
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${UNITS[i]}`;
}

/** Bytes per second → [value, unit] with a per-second unit, e.g. ['4.0', 'MB/s']. */
export function rate(bps: number): [string, string] {
  const [v, u] = bytes(bps).split(' ');
  return [v ?? '0', `${u ?? UNITS[0]}${PER_SECOND}`];
}

/** 6 d 2 h · 3 h 12 m · 45 m. */
export function uptime(s: number, d = 'd', h = 'h', m = 'm'): string {
  const days = Math.floor(s / 86400);
  const hours = Math.floor((s % 86400) / 3600);
  const mins = Math.floor((s % 3600) / 60);
  if (days) return hours ? `${days} ${d} ${hours} ${h}` : `${days} ${d}`;
  if (hours) return `${hours} ${h} ${mins} ${m}`;
  return `${mins} ${m}`;
}

/** Missing samples are distinct from zero load. */
export const pct = (v: number | null): string =>
  v == null ? '—' : `${Math.round(Math.max(0, Math.min(100, v)))}`;

/** Percent label without inventing a unit for missing data. */
export const percent = (v: number | null): string => (v == null ? '—' : `${pct(v)}%`);

/** Load average or process CPU, retaining the requested precision. */
export const decimal = (v: number | null, digits = 2): string =>
  v == null ? '—' : v.toFixed(digits);

/** How busy: below 60 % calm, from 60 % busy, from 85 % hot; `null` when unknown. */
export function loadTone(p: number | null): 'calm' | 'warm' | 'hot' | null {
  if (p == null) return null;
  return p >= 85 ? 'hot' : p >= 60 ? 'warm' : 'calm';
}

/** Ring / bar colour by load, the same scale on every screen. */
export function loadColor(p: number | null): string {
  const t = loadTone(p);
  return t ? `var(--load-${t})` : 'var(--muted)';
}

/** "5 min ago" in the given locale, rounded to the largest whole unit. */
export function ago(ts: number, locale: string, now = Date.now()): string {
  const s = Math.round((ts - now) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto', style: 'short' });
  const a = Math.abs(s);
  if (a < 60) return rtf.format(0, 'second');
  if (a < 3600) return rtf.format(Math.round(s / 60), 'minute');
  if (a < 86400) return rtf.format(Math.round(s / 3600), 'hour');
  return rtf.format(Math.round(s / 86400), 'day');
}

/** TCP connect time; 0 means under a millisecond (often a local proxy/VPN answering first). */
export const ms = (v: number): string => (v < 1 ? '<1 ms' : `${v} ms`);

/** Picks the plural form for `n` in `locale` (`one` / `few` / `many`; English uses one / many). */
export function pluralForm(n: number, locale: string): 'one' | 'few' | 'many' {
  const cat = new Intl.PluralRules(locale).select(n);
  return cat === 'one' ? 'one' : cat === 'few' ? 'few' : 'many';
}
