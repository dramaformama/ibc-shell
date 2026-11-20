import { describe, expect, it } from 'vitest';
import type { Folder, Host, Tree } from '$lib/api';
import { hostComparator, type SortFields } from './host-sort';
import { buildRows } from './tree-model';

const f = (label: string, flag: string | null = null, last: number | null = null): SortFields => ({
  label,
  flag,
  last
});
const order = (
  sort: Parameters<typeof hostComparator>[0],
  items: SortFields[],
  locale: 'ru' | 'en' = 'en'
) => {
  const cmp = hostComparator(sort, locale);
  return cmp ? [...items].sort(cmp).map((x) => x.label) : items.map((x) => x.label);
};

describe('hostComparator', () => {
  it('manual keeps the given order', () => {
    expect(hostComparator('manual', 'en')).toBeNull();
  });
  it('sorts names naturally and case-insensitively', () => {
    expect(order('name', [f('web-10'), f('Web-2'), f('api')])).toEqual(['api', 'Web-2', 'web-10']);
  });
  it('puts the most recent login first and never-connected last', () => {
    expect(order('last', [f('a'), f('b', null, 100), f('c', null, 300), f('d')])).toEqual([
      'c',
      'b',
      'a',
      'd'
    ]);
  });
  it('sorts by localized country name, flagless last', () => {
    // en: Germany < Netherlands < United States; ru: Германия < Нидерланды < США
    const items = [f('x'), f('us1', 'us'), f('nl1', 'nl'), f('de1', 'de'), f('de0', 'de')];
    expect(order('country', items)).toEqual(['de0', 'de1', 'nl1', 'us1', 'x']);
    expect(order('country', items, 'ru')).toEqual(['de0', 'de1', 'nl1', 'us1', 'x']);
  });
});

const folder = (id: string, parent: string | null, sort: string, name = id): Folder =>
  ({
    id,
    parent_id: parent,
    name,
    country_code: id === 'eu' ? 'nl' : null,
    sort_key: sort,
    expanded: true
  }) as Folder;
const host = (id: string, parent: string | null, sort: string, over: Partial<Host> = {}): Host =>
  ({
    id,
    folder_id: parent,
    label: id,
    hostname: id + '.example',
    sort_key: sort,
    country_code: null,
    last_connected_at: null,
    tag_ids: [],
    ...over
  }) as Host;

describe('buildRows with a sort', () => {
  const tree: Tree = {
    folders: [folder('zz', null, 'a'), folder('eu', null, 'c')],
    hosts: [
      host('b', null, 'b', { last_connected_at: 5 }),
      host('a', null, 'd', { country_code: 'us' }),
      host('n2', 'eu', 'a', { last_connected_at: 9 }),
      host('n1', 'eu', 'b', { country_code: 'de' })
    ]
  };
  it('manual order is untouched', () => {
    expect(buildRows(tree).map((r) => r.id)).toEqual(['zz', 'b', 'eu', 'n2', 'n1', 'a']);
  });
  it('name: folders first by name, hosts by name in each folder', () => {
    expect(buildRows(tree, 'name').map((r) => r.id)).toEqual(['eu', 'n1', 'n2', 'zz', 'a', 'b']);
  });
  it('last login inside folders', () => {
    expect(buildRows(tree, 'last').map((r) => r.id)).toEqual(['eu', 'n2', 'n1', 'zz', 'b', 'a']);
  });
  it('country uses the inherited folder flag', () => {
    // n1 = Germany, n2 inherits Netherlands from the folder; root: a = US, b has no flag.
    expect(buildRows(tree, 'country').map((r) => r.id)).toEqual(['eu', 'n1', 'n2', 'zz', 'a', 'b']);
  });
});
