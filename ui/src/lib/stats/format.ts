/** Number formatting for the status cards (compact, Apple-style units). */

const UNITS = ['B', 'K', 'M', 'G', 'T'];

/** 16.0 G, 452 M, 812 K. */
export function bytes(n: number): string {
  let v = Math.max(0, n);
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${UNITS[i]}`;
}

/** Bytes per second → [value, unit] with a /s unit, e.g. ['4.01', 'M/s']. */
export function rate(bps: number): [string, string] {
  const [v, u] = bytes(bps).split(' ');
  return [v ?? '0', `${u ?? 'B'}/s`];
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

export const pct = (v: number): string => `${Math.round(Math.max(0, Math.min(100, v)))}`;

/** Ring / bar colour by load: calm → busy → hot. */
export function loadColor(p: number): string {
  if (p >= 85) return 'var(--danger)';
  if (p >= 60) return 'var(--c-yellow)';
  return 'var(--c-cyan)';
}
