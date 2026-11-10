import { beforeEach, describe, expect, it } from 'vitest';
import { groupKey, sessions, type Tab } from './sessions.svelte';

const tab = (hostId: string, kind: Tab['kind'] = 'terminal'): Omit<Tab, 'id'> => ({
  kind,
  hostId,
  label: hostId,
  flag: null,
  adhoc: null
});

describe('background connections', () => {
  beforeEach(() => sessions.clear());

  it('keeps servers opened for monitoring out of the tab bar', () => {
    for (const h of ['a', 'b', 'c']) sessions.open(tab(h), false);
    expect(sessions.groups).toHaveLength(3);
    expect(sessions.visibleGroups).toHaveLength(0);
    expect(sessions.activeId).toBeNull();
  });

  it('shows a server once the user opens it', () => {
    const a = sessions.open(tab('a'), false);
    sessions.open(tab('b'), false);
    sessions.select(a.id);
    expect(sessions.visibleGroups.map((g) => g.label)).toEqual(['a']);
  });

  it('shows a server whose status view was opened over a hidden terminal', () => {
    sessions.open(tab('a'), false);
    sessions.open(tab('a', 'status'), true);
    expect(sessions.visibleGroups).toHaveLength(1);
    expect(sessions.groups[0]?.tabs).toHaveLength(2);
  });

  it('falls back to a visible tab, never a hidden one, when the active one closes', () => {
    const a = sessions.open(tab('a'), true);
    sessions.open(tab('bg'), false);
    const c = sessions.open(tab('c'), true);
    sessions.close(c.id);
    expect(sessions.activeId).toBe(a.id);
    sessions.close(a.id);
    expect(sessions.activeId).toBeNull();
  });

  it('steps only through visible servers', () => {
    const a = sessions.open(tab('a'), true);
    sessions.open(tab('bg'), false);
    const c = sessions.open(tab('c'), true);
    sessions.select(a.id);
    sessions.step(1);
    expect(sessions.activeId).toBe(c.id);
  });

  it('hides a server from the tab bar but keeps its terminal connected', () => {
    const a = sessions.open(tab('a'), true);
    sessions.open(tab('a', 'status'), true);
    const b = sessions.open(tab('b'), true);
    sessions.select(a.id);
    sessions.hideGroup('h:a');
    expect(sessions.visibleGroups.map((g) => g.label)).toEqual(['b']);
    expect(sessions.groups.find((g) => g.label === 'a')?.tabs.map((t) => t.kind)).toEqual([
      'terminal'
    ]);
    expect(sessions.activeId).toBe(b.id);
    sessions.hideGroup('h:b');
    expect(sessions.activeId).toBeNull();
    expect(sessions.groups).toHaveLength(2);
  });
});

const base = { hostId: 'h1', label: 'a', flag: null, adhoc: null } as Omit<Tab, 'id' | 'kind'>;

describe('several windows of one host', () => {
  beforeEach(() => sessions.clear());

  it('keeps each window in its own group', () => {
    sessions.open({ ...base, kind: 'terminal', instance: 1 });
    sessions.open({ ...base, kind: 'terminal', instance: 2 });
    sessions.open({ ...base, kind: 'sftp', instance: 2 });
    expect(sessions.groups.map((g) => g.tabs.length)).toEqual([1, 2]);
    expect(sessions.find('h1', 'sftp', 1)).toBeNull();
    expect(sessions.find('h1', 'sftp', 2)?.instance).toBe(2);
  });

  it('reuses the lowest free window number', () => {
    const one = sessions.open({ ...base, kind: 'terminal', instance: 1 });
    sessions.open({ ...base, kind: 'terminal', instance: 2 });
    expect(sessions.nextInstance('h1')).toBe(3);
    sessions.close(one.id);
    expect(sessions.nextInstance('h1')).toBe(1);
  });

  it('leaves the first window key unchanged', () => {
    const t = sessions.open({ ...base, kind: 'terminal' });
    expect(groupKey(t)).toBe('h:h1');
  });
});

describe('tab order (drag and drop)', () => {
  beforeEach(() => sessions.clear());
  const labels = () => sessions.visibleGroups.map((g) => g.label);

  it('moves a server and keeps its views together', () => {
    for (const h of ['a', 'b', 'c']) sessions.open(tab(h));
    sessions.open(tab('a', 'sftp'));
    sessions.moveGroup(groupKey(sessions.find('a', 'terminal')!), 2);
    expect(labels()).toEqual(['b', 'c', 'a']);
    expect(sessions.groups.find((g) => g.label === 'a')?.tabs.map((t) => t.kind)).toEqual([
      'terminal',
      'sftp'
    ]);
  });

  it('moves left, clamps out-of-range targets, ignores unknown keys', () => {
    for (const h of ['a', 'b', 'c']) sessions.open(tab(h));
    sessions.moveGroup('h:c', 0);
    expect(labels()).toEqual(['c', 'a', 'b']);
    sessions.moveGroup('h:c', 99);
    expect(labels()).toEqual(['a', 'b', 'c']);
    sessions.moveGroup('nope', 0);
    expect(labels()).toEqual(['a', 'b', 'c']);
  });

  it('counts positions among shown servers only', () => {
    sessions.open(tab('a'));
    sessions.open(tab('hidden'), false);
    sessions.open(tab('b'));
    sessions.moveGroup('h:b', 0);
    expect(labels()).toEqual(['b', 'a']);
    expect(sessions.groups).toHaveLength(3);
  });
});

describe('local terminal', () => {
  beforeEach(() => sessions.clear());

  it('is started once, in the background, and is not a server', () => {
    const a = sessions.ensureLocal('Mac');
    expect(sessions.ensureLocal('Mac').id).toBe(a.id);
    expect(sessions.activeId).toBeNull();
    expect(sessions.groups).toHaveLength(0);
    sessions.openLocal('Mac');
    expect(sessions.activeId).toBe(a.id);
    expect(sessions.visibleGroups).toHaveLength(0);
    expect(sessions.activeGroup).toBeNull();
  });

  it('survives closing servers and is never picked as the next tab', () => {
    const local = sessions.ensureLocal('Mac');
    const a = sessions.open(tab('a'));
    sessions.closeGroup(groupKey(local));
    sessions.close(a.id);
    expect(sessions.localTab?.id).toBe(local.id);
    expect(sessions.activeId).toBeNull();
  });

  it('stays last when servers are reordered', () => {
    sessions.ensureLocal('Mac');
    sessions.open(tab('a'));
    sessions.open(tab('b'));
    sessions.moveGroup('h:b', 0);
    expect(sessions.tabs.at(-1)?.local).toBe(true);
  });
});

describe('activity marks', () => {
  beforeEach(() => sessions.clear());

  it('marks only tabs in the background and clears on open', () => {
    const a = sessions.open(tab('a'));
    const b = sessions.open(tab('b'));
    sessions.markActivity(b.id);
    expect(sessions.activity[b.id]).toBeUndefined();
    sessions.markActivity(a.id);
    const ga = sessions.groups.find((g) => g.label === 'a')!;
    expect(sessions.groupActivity(ga)).toBe(true);
    sessions.select(a.id);
    expect(sessions.activity[a.id]).toBeUndefined();
  });
});
