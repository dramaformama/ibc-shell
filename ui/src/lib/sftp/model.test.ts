import { describe, expect, it } from 'vitest';
import type { Entry } from '$lib/api';
import {
  crumbs,
  emptySelection,
  ema,
  etaSeconds,
  filterEntries,
  formatEta,
  formatMode,
  formatSize,
  joinPath,
  parentPath,
  parseOctal,
  prune,
  select,
  sortEntries,
  toOctal
} from './model';

const e = (
  name: string,
  kind: Entry['kind'],
  size = 0,
  mtime: number | null = 0,
  mode = 0o644
): Entry => ({ name, kind, size, mtime, mode, link_target: null });

describe('sortEntries', () => {
  const list = [
    e('b.txt', 'file', 5),
    e('a.txt', 'file', 9),
    e('zdir', 'dir'),
    e('file10', 'file'),
    e('file2', 'file')
  ];
  it('puts folders first and sorts naturally by name', () => {
    expect(sortEntries(list, { key: 'name', dir: 'asc' }).map((x) => x.name)).toEqual([
      'zdir',
      'a.txt',
      'b.txt',
      'file2',
      'file10'
    ]);
  });
  it('keeps folders first when descending', () => {
    const r = sortEntries(list, { key: 'size', dir: 'desc' });
    expect(r[0]?.name).toBe('zdir');
    expect(r[1]?.name).toBe('a.txt');
  });
  it('does not mutate input', () => {
    const copy = [...list];
    sortEntries(list, { key: 'name', dir: 'desc' });
    expect(list).toEqual(copy);
  });
});

describe('filterEntries', () => {
  const list = [e('.env', 'file'), e('Readme', 'file')];
  it('hides dotfiles and filters', () => {
    expect(filterEntries(list, '', false).map((x) => x.name)).toEqual(['Readme']);
    expect(filterEntries(list, 'ENV', true).map((x) => x.name)).toEqual(['.env']);
  });
});

describe('formatting', () => {
  it('formats binary sizes', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(1536)).toBe('1.5 KiB');
    expect(formatSize(1024 * 1024 * 20)).toBe('20 MiB');
  });
  it('formats permissions', () => {
    expect(formatMode(0o100644, 'file')).toBe('-rw-r--r--');
    expect(formatMode(0o40755, 'dir')).toBe('drwxr-xr-x');
    expect(formatMode(0o4755, 'file')).toBe('-rwsr-xr-x');
    expect(formatMode(0o1777, 'dir')).toBe('drwxrwxrwt');
    expect(formatMode(0)).toBe('');
  });
  it('octal round trip', () => {
    expect(toOctal(0o100644)).toBe('644');
    expect(parseOctal('755')).toBe(0o755);
    expect(parseOctal('889')).toBeNull();
  });
  it('formats ETA', () => {
    expect(formatEta(42)).toBe('42s');
    expect(formatEta(125)).toBe('2m 05s');
    expect(formatEta(null)).toBe('');
  });
});

describe('selection', () => {
  const order = ['a', 'b', 'c', 'd'];
  it('replaces, toggles and ranges', () => {
    let s = select(emptySelection, order, 'b');
    expect([...s.names]).toEqual(['b']);
    s = select(s, order, 'd', { shift: true });
    expect([...s.names]).toEqual(['b', 'c', 'd']);
    s = select(s, order, 'c', { meta: true });
    expect([...s.names]).toEqual(['b', 'd']);
    s = select(s, order, 'a');
    expect([...s.names]).toEqual(['a']);
  });
  it('prunes vanished names', () => {
    const s = select(emptySelection, order, 'a', {});
    expect(prune(s, ['b']).names.size).toBe(0);
  });
});

describe('paths', () => {
  it('joins and walks up', () => {
    expect(joinPath('/', 'x')).toBe('/x');
    expect(joinPath('/a', 'x')).toBe('/a/x');
    expect(parentPath('/a/b')).toBe('/a');
    expect(parentPath('/a')).toBe('/');
    expect(parentPath('/')).toBe('/');
  });
  it('builds crumbs', () => {
    expect(crumbs('/a/b')).toEqual([
      { label: '/', path: '/' },
      { label: 'a', path: '/a' },
      { label: 'b', path: '/a/b' }
    ]);
  });
});

describe('transfer math', () => {
  it('smooths speed', () => {
    expect(ema(null, 100)).toBe(100);
    expect(ema(100, 200)).toBeCloseTo(130);
  });
  it('estimates ETA', () => {
    expect(etaSeconds(1000, 0, 100)).toBe(10);
    expect(etaSeconds(1000, 1000, 100)).toBeNull();
    expect(etaSeconds(1000, 0, 0)).toBeNull();
  });
});
