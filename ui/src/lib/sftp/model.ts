/** Pure helpers for the SFTP screen: sorting, formatting, selection, paths, transfer math. */
import type { Endpoint, Entry } from '$lib/api';

/** A choosable pane endpoint (Local or an open SFTP session). */
export interface Source {
  value: string;
  label: string;
  ep: Endpoint;
}

/** Everything a pane's table can ask its owner to do. */
export interface Actions {
  open(e: Entry): void;
  rename(e: Entry): void;
  remove(): void;
  perms(e: Entry): void;
  copyPath(e: Entry): void;
  newFolder(): void;
  transfer(): void;
  transferLabel: string;
  canPerms: boolean;
  canTransfer: boolean;
}

export type SortKey = 'name' | 'size' | 'mtime' | 'mode';
export interface Sort {
  key: SortKey;
  dir: 'asc' | 'desc';
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/** Folders first, then by the chosen key; name is the stable tiebreaker. Never mutates the input. */
export function sortEntries(entries: readonly Entry[], sort: Sort): Entry[] {
  const sign = sort.dir === 'asc' ? 1 : -1;
  const rank = (e: Entry) => (e.kind === 'dir' ? 0 : 1);
  return [...entries].sort((a, b) => {
    if (rank(a) !== rank(b)) return rank(a) - rank(b);
    let c: number;
    if (sort.key === 'size') c = a.size - b.size;
    else if (sort.key === 'mtime') c = (a.mtime ?? 0) - (b.mtime ?? 0);
    else if (sort.key === 'mode') c = (a.mode & 0o7777) - (b.mode & 0o7777);
    else c = collator.compare(a.name, b.name);
    return c !== 0 ? c * sign : collator.compare(a.name, b.name);
  });
}

/** Case-insensitive substring filter, optional dotfile hiding. */
export function filterEntries(
  entries: readonly Entry[],
  query: string,
  showHidden: boolean
): Entry[] {
  const q = query.trim().toLowerCase();
  return entries.filter(
    (e) => (showHidden || !e.name.startsWith('.')) && (!q || e.name.toLowerCase().includes(q))
  );
}

const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
/** Binary units, one decimal under 10, none above. */
export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 || v >= 10 ? 0 : 1;
  return `${v.toFixed(digits)} ${UNITS[i]}`;
}

/** `drwxr-xr-x` (type letter + rwx triplets, setuid/setgid/sticky aware). Unknown mode (0) → ''. */
export function formatMode(mode: number, kind: Entry['kind'] = 'file'): string {
  if (!mode) return '';
  const t = kind === 'dir' ? 'd' : kind === 'symlink' ? 'l' : kind === 'other' ? '?' : '-';
  const chars = 'rwxrwxrwx'.split('');
  chars.forEach((ch, i) => {
    if (!(mode & (1 << (8 - i)))) chars[i] = '-';
    else chars[i] = ch;
  });
  const special = (flag: number, at: number, ch: string) => {
    if (!(mode & flag)) return;
    chars[at] = chars[at] === 'x' ? ch : ch.toUpperCase();
  };
  special(0o4000, 2, 's');
  special(0o2000, 5, 's');
  special(0o1000, 8, 't');
  return t + chars.join('');
}

/** Permission bits as 3–4 digit octal text. */
export const toOctal = (mode: number): string => (mode & 0o7777).toString(8).padStart(3, '0');
export function parseOctal(text: string): number | null {
  const t = text.trim();
  return /^[0-7]{3,4}$/.test(t) ? parseInt(t, 8) : null;
}

const dateFmt = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit'
});
export const formatDate = (ms: number | null): string => (ms == null ? '' : dateFmt.format(ms));

// ---- selection ----
export interface Selection {
  names: ReadonlySet<string>;
  anchor: string | null;
}
export const emptySelection: Selection = { names: new Set(), anchor: null };

/** Click semantics: plain = replace, ⌘/Ctrl = toggle, ⇧ = range from anchor over `order`. */
export function select(
  cur: Selection,
  order: readonly string[],
  name: string,
  mods: { shift?: boolean; meta?: boolean } = {}
): Selection {
  if (mods.shift && cur.anchor !== null) {
    const a = order.indexOf(cur.anchor);
    const b = order.indexOf(name);
    if (a !== -1 && b !== -1) {
      const range = order.slice(Math.min(a, b), Math.max(a, b) + 1);
      return { names: new Set(mods.meta ? [...cur.names, ...range] : range), anchor: cur.anchor };
    }
  }
  if (mods.meta) {
    const next = new Set(cur.names);
    if (!next.delete(name)) next.add(name);
    return { names: next, anchor: name };
  }
  return { names: new Set([name]), anchor: name };
}

/** Keeps only names that still exist (after refresh or filter). */
export function prune(sel: Selection, order: readonly string[]): Selection {
  const have = new Set(order);
  const names = new Set([...sel.names].filter((n) => have.has(n)));
  if (names.size === sel.names.size) return sel;
  return { names, anchor: sel.anchor && have.has(sel.anchor) ? sel.anchor : null };
}

// ---- paths (POSIX; Windows-style local paths use backslashes) ----
const sepOf = (p: string): string => (p.includes('\\') && !p.includes('/') ? '\\' : '/');

export function joinPath(dir: string, name: string): string {
  const s = sepOf(dir);
  return dir.endsWith(s) ? dir + name : dir + s + name;
}
export function parentPath(path: string): string {
  const s = sepOf(path);
  const parts = path.split(s).filter(Boolean);
  if (s === '/') return '/' + parts.slice(0, -1).join('/');
  return parts.length <= 1 ? path : parts.slice(0, -1).join(s) + (parts.length === 2 ? s : '');
}
export function crumbs(path: string): { label: string; path: string }[] {
  const s = sepOf(path);
  const out = s === '/' ? [{ label: '/', path: '/' }] : [];
  let acc = '';
  path
    .split(s)
    .filter(Boolean)
    .forEach((p, i) => {
      acc = s === '/' ? `${acc}/${p}` : i === 0 ? p + s : joinPath(acc, p);
      out.push({ label: p, path: acc });
    });
  return out;
}

// ---- transfers ----
/** Exponential moving average; first sample seeds it. */
export const ema = (prev: number | null, sample: number, alpha = 0.3): number =>
  prev === null || prev === 0 ? sample : prev + alpha * (sample - prev);

/** Seconds left, or null when unknowable. */
export function etaSeconds(total: number, done: number, speed: number): number | null {
  if (speed <= 0 || total <= done) return null;
  return Math.ceil((total - done) / speed);
}
export function formatEta(sec: number | null): string {
  if (sec === null) return '';
  if (sec < 60) return `${sec}s`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m ${String(sec % 60).padStart(2, '0')}s`;
  return `${Math.floor(sec / 3600)}h ${String(Math.floor((sec % 3600) / 60)).padStart(2, '0')}m`;
}
export const formatSpeed = (bps: number): string => `${formatSize(bps)}/s`;
