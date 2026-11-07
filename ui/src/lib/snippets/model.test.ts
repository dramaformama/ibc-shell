import { describe, expect, it } from 'vitest';
import {
  emptyLayout,
  fillVars,
  frequent,
  keystrokes,
  moveSnippet,
  parseVars,
  readLayout,
  sections
} from './model';

const s = (id: string, name = id, body = 'echo ' + id) => ({ id, name, body });

describe('snippet variables', () => {
  it('finds distinct placeholders with defaults', () => {
    expect(parseVars('docker logs {{ name }} --tail {{lines:100}} {{name}}')).toEqual([
      { name: 'name', fallback: '' },
      { name: 'lines', fallback: '100' }
    ]);
  });
  it('fills values and falls back to defaults', () => {
    expect(fillVars('tail -n {{n:50}} {{file}}', { file: '/var/log/syslog' })).toBe(
      'tail -n 50 /var/log/syslog'
    );
  });
});

describe('keystrokes', () => {
  it('turns lines into Enter presses and executes', () => {
    expect(keystrokes('cd /srv\nls -la\n', true)).toBe('cd /srv\rls -la\r');
  });
  it('leaves the last line on the prompt when inserting', () => {
    expect(keystrokes('git commit -m ""', false)).toBe('git commit -m ""');
  });
});

describe('snippet folders', () => {
  const items = [s('a', 'Logs'), s('b', 'Disk'), s('c', 'Restart nginx')];
  it('puts everything under "No folder" without groups', () => {
    const out = sections(items, emptyLayout());
    expect(out).toHaveLength(1);
    expect(out[0]!.items.map((x) => x.name)).toEqual(['Disk', 'Logs', 'Restart nginx']);
  });
  it('groups, orders and filters', () => {
    const l = { ...emptyLayout(), groups: [{ id: 'g', name: 'Web' }], of: { c: 'g' } };
    const moved = moveSnippet(l, ['a', 'b', 'c'], 'a', 'g', 'c');
    const out = sections(items, moved);
    expect(out[0]!.items.map((x) => x.id)).toEqual(['a', 'c']);
    expect(out[1]!.items.map((x) => x.id)).toEqual(['b']);
    expect(sections(items, moved, 'nginx').map((x) => x.items.map((i) => i.id))).toEqual([['c']]);
  });
  it('moves to the end of a group and out of it', () => {
    let l = { ...emptyLayout(), groups: [{ id: 'g', name: 'Web' }] };
    l = moveSnippet(l, ['a', 'b', 'c'], 'b', 'g', null);
    l = moveSnippet(l, ['a', 'b', 'c'], 'c', 'g', null);
    expect(sections(items, l)[0]!.items.map((x) => x.id)).toEqual(['b', 'c']);
    l = moveSnippet(l, ['a', 'b', 'c'], 'b', null, null);
    expect(l.of.b).toBeUndefined();
  });
  it('ranks frequent snippets', () => {
    const l = { ...emptyLayout(), uses: { a: 5, b: 1, c: 3 } };
    expect(frequent(items, l).map((x) => x.id)).toEqual(['a', 'c']);
  });
  it('ignores malformed stored layouts', () => {
    const l = readLayout({ groups: [{ id: 1 }, { id: 'g', name: 'Ok' }], of: { a: 3 }, uses: 'x' });
    expect(l.groups).toEqual([{ id: 'g', name: 'Ok' }]);
    expect(l.of).toEqual({});
    expect(l.uses).toEqual({});
    expect(readLayout(null)).toEqual(emptyLayout());
  });
});
