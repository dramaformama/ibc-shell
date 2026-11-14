/** Tiny in-memory POSIX file system used by the mock SFTP and local panes. */
import type { DirListing, Entry, EntryKind } from '../types';

export interface Node {
  kind: EntryKind;
  size: number;
  mtime: number;
  mode: number;
  link_target: string | null;
}

export const normalize = (p: string): string => {
  const parts: string[] = [];
  for (const seg of p.split('/')) {
    if (!seg || seg === '.') continue;
    if (seg === '..') parts.pop();
    else parts.push(seg);
  }
  return `/${parts.join('/')}`;
};

export const dirname = (p: string): string => normalize(`${p}/..`);
export const basename = (p: string): string => p.split('/').filter(Boolean).at(-1) ?? '';
export const join = (dir: string, name: string): string => normalize(`${dir}/${name}`);

export class MemFs {
  readonly nodes = new Map<string, Node>([['/', dir()]]);

  constructor(public readonly home: string) {}

  add(path: string, node: Node): void {
    const p = normalize(path);
    const parent = dirname(p);
    if (parent !== p && !this.nodes.has(parent)) this.add(parent, dir());
    this.nodes.set(p, node);
  }

  get(path: string): Node | undefined {
    return this.nodes.get(normalize(path));
  }

  entry(path: string): Entry | null {
    const n = this.get(path);
    if (!n) return null;
    return {
      name: basename(path),
      kind: n.kind,
      size: n.size,
      mtime: n.mtime,
      mode: n.mode,
      link_target: n.link_target
    };
  }

  list(path: string): DirListing | null {
    const p = normalize(path);
    if (this.get(p)?.kind !== 'dir') return null;
    const entries: Entry[] = [];
    for (const key of this.nodes.keys()) {
      if (key !== p && dirname(key) === p) entries.push(this.entry(key)!);
    }
    return { path: p, entries };
  }

  /** Paths under `path` (inclusive). */
  subtree(path: string): string[] {
    const p = normalize(path);
    return [...this.nodes.keys()].filter((k) => k === p || k.startsWith(p === '/' ? '/' : `${p}/`));
  }

  remove(path: string): void {
    for (const k of this.subtree(path)) this.nodes.delete(k);
  }

  rename(from: string, to: string): void {
    const src = normalize(from);
    const dst = normalize(to);
    for (const k of this.subtree(src)) {
      const n = this.nodes.get(k)!;
      this.nodes.delete(k);
      this.nodes.set(dst + k.slice(src.length), n);
    }
  }

  /** Total bytes and file count under `path`. */
  measure(path: string): { bytes: number; files: number } {
    let bytes = 0;
    let files = 0;
    for (const k of this.subtree(path)) {
      const n = this.nodes.get(k)!;
      if (n.kind === 'file') {
        bytes += n.size;
        files += 1;
      }
    }
    return { bytes, files };
  }
}

export function dir(): Node {
  return {
    kind: 'dir',
    size: 4096,
    mtime: Date.now() - 86_400_000,
    mode: 0o40755,
    link_target: null
  };
}

export function file(size: number, mode = 0o100644): Node {
  return {
    kind: 'file',
    size,
    mtime: Date.now() - Math.floor(Math.random() * 30) * 86_400_000,
    mode,
    link_target: null
  };
}

export function remoteSeed(): MemFs {
  const fs = new MemFs('/root');
  fs.add('/root', { ...dir(), mode: 0o40700 });
  fs.add('/root/.bashrc', file(3_771));
  fs.add('/root/backup.tar.gz', file(734_003_200));
  fs.add('/etc/hosts', file(221));
  fs.add('/etc/nginx/nginx.conf', file(2_412));
  fs.add('/var/log/syslog', file(5_242_880, 0o100640));
  fs.add('/var/log/auth.log', file(17_825_792, 0o100640));
  fs.add('/srv/www/index.html', file(14_200));
  fs.add('/srv/current', {
    kind: 'symlink',
    size: 0,
    mtime: Date.now(),
    mode: 0o120777,
    link_target: '/srv/www'
  });
  // Hostile / unusual names for UI + sanitizer testing (rendered as plain text only).
  for (const n of [
    '..evil',
    'CON',
    'name with spaces.txt',
    'юникод файл.txt',
    'trailing dot.',
    '<script>.txt'
  ]) {
    fs.add(`/srv/weird/${n}`, file(42));
  }
  for (let i = 0; i < 1000; i++)
    fs.add(`/srv/many/file-${String(i).padStart(4, '0')}.log`, file(1024 + i));
  return fs;
}

export function localSeed(): MemFs {
  const fs = new MemFs('/Users/demo');
  fs.add('/Users/demo/Documents/report.pdf', file(1_234_567));
  fs.add('/Users/demo/Downloads/ubuntu.iso', file(2_147_483_648));
  fs.add('/Users/demo/notes.txt', file(2_048));
  fs.add('/Users/demo/photo.jpg', file(3_500_000));
  return fs;
}
