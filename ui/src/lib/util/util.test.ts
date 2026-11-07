import { describe, expect, it } from 'vitest';
import type { Folder, Host, Tree } from '$lib/api';
import { fuzzyScore, rank } from './fuzzy';
import { parseQuickConnect } from './quickconnect';
import { passwordStrength } from './strength';
import {
  applyMoveLocal,
  buildRows,
  fkey,
  inheritedDefault,
  inheritedFlag,
  resolveDrop,
  hkey
} from './tree-model';

const folder = (
  id: string,
  parent: string | null,
  sort: string,
  over: Partial<Folder> = {}
): Folder =>
  ({
    id,
    parent_id: parent,
    name: id,
    country_code: null,
    color: null,
    icon: null,
    sort_key: sort,
    expanded: true,
    defaults: {
      username: null,
      port: null,
      identity_id: null,
      jump_host_id: null,
      startup_cmd: null,
      keepalive_s: null,
      term_theme: null
    },
    created_at: 0,
    updated_at: 0,
    ...over
  }) as Folder;
const host = (id: string, parent: string | null, sort: string, over: Partial<Host> = {}): Host =>
  ({
    id,
    folder_id: parent,
    label: id,
    hostname: id + '.example',
    sort_key: sort,
    country_code: null,
    tag_ids: [],
    ...over
  }) as Host;

const tree: Tree = {
  folders: [
    folder('de', null, 'a', {
      country_code: 'de',
      defaults: { username: 'root' } as Folder['defaults']
    }),
    folder('db', 'de', 'b')
  ],
  hosts: [host('h1', 'de', 'a'), host('h2', 'db', 'a'), host('h3', null, 'z')]
};

describe('fuzzyScore', () => {
  it('matches subsequences and rejects misses', () => {
    expect(fuzzyScore('dfr', 'de-fra-1')).toBeGreaterThan(0);
    expect(fuzzyScore('xyz', 'de-fra-1')).toBe(0);
  });
  it('prefers prefix/substring over scattered', () => {
    expect(fuzzyScore('fra', 'fra-1')).toBeGreaterThan(fuzzyScore('fra', 'f-r-a-1'));
  });
  it('ranks best first', () => {
    expect(rank('nl', ['de-fra', 'nl-ams', 'nlx'], (s) => [s])[0]).toBe('nlx');
  });
});

describe('parseQuickConnect', () => {
  it('parses user@host:port', () => {
    expect(parseQuickConnect('root@1.2.3.4:2222')).toEqual({
      user: 'root',
      hostname: '1.2.3.4',
      port: 2222
    });
  });
  it('parses host only and ipv6', () => {
    expect(parseQuickConnect('example.com')).toEqual({
      user: null,
      hostname: 'example.com',
      port: null
    });
    expect(parseQuickConnect('me@[::1]:22')).toEqual({ user: 'me', hostname: '::1', port: 22 });
  });
  it('rejects junk', () => {
    expect(parseQuickConnect('fra')).toBeNull();
    expect(parseQuickConnect('a b@c')).toBeNull();
    expect(parseQuickConnect('x@h:99999')).toBeNull();
    expect(parseQuickConnect('@host')).toBeNull();
  });
});

describe('passwordStrength', () => {
  it('grows with quality', () => {
    expect(passwordStrength('')).toBe(0);
    expect(passwordStrength('abc')).toBeLessThanOrEqual(1);
    expect(passwordStrength('correct Horse battery 7!')).toBeGreaterThanOrEqual(3);
    expect(passwordStrength('aaaaaaaaaaaa')).toBeLessThanOrEqual(1);
  });
});

describe('tree model', () => {
  it('builds ordered rows with counts and inherited flags', () => {
    const rows = buildRows(tree);
    expect(rows.map((r) => r.key)).toEqual(['f:de', 'h:h1', 'f:db', 'h:h2', 'h:h3']);
    expect(rows[0]?.count).toBe(2);
    expect(rows[1]?.flag).toBe('de');
    expect(rows[4]?.flag).toBeNull();
  });
  it('hides collapsed subtrees', () => {
    const t = {
      ...tree,
      folders: tree.folders.map((f) => (f.id === 'de' ? { ...f, expanded: false } : f))
    };
    expect(buildRows(t).map((r) => r.key)).toEqual(['f:de', 'h:h3']);
  });
  it('looks up inherited defaults and flags', () => {
    expect(inheritedDefault(tree, 'db', 'username')).toEqual({ value: 'root', from: 'de' });
    expect(inheritedDefault(tree, 'db', 'port')).toBeNull();
    expect(inheritedFlag(tree, 'db')?.value).toBe('de');
  });
  it('refuses to drop a folder into its own subtree', () => {
    const rows = buildRows(tree);
    expect(resolveDrop(tree, rows, 2, 'inside', new Set([fkey('de')]))).toBeNull();
    expect(resolveDrop(tree, rows, 4, 'inside', new Set([hkey('h1')]))).toBeNull();
  });
  it('resolves before/after to parent + anchor', () => {
    const rows = buildRows(tree);
    const d = resolveDrop(tree, rows, 4, 'before', new Set([hkey('h1')]));
    expect(d).toMatchObject({ parent: null, after: { kind: 'folder', id: 'de' } });
    const inside = resolveDrop(tree, rows, 0, 'after', new Set([hkey('h3')]));
    expect(inside).toMatchObject({ parent: 'de', after: null });
  });
  it('applies moves locally', () => {
    const t = applyMoveLocal(tree, [
      { ref: { kind: 'host', id: 'h3' }, parent: 'de', after: null }
    ]);
    expect(t.hosts.find((h) => h.id === 'h3')?.folder_id).toBe('de');
    expect(buildRows(t)[1]?.key).toBe('h:h3');
  });
});
