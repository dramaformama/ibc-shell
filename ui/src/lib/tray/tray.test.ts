import { describe, expect, it } from 'vitest';
import type { HostStats } from '$lib/api';
import { buildMenu, line } from './tray.svelte';

const d = (over: Partial<HostStats>) =>
  ({
    cpu_pct: 12,
    mem_total: 100,
    mem_used: 40,
    disk_total: 100,
    disk_used: 50,
    ...over
  }) as HostStats;

describe('tray', () => {
  it('describes each state', () => {
    const base = { hostId: 'h', label: 'DE01' };
    expect(line({ ...base, phase: 'connected', d: d({}) })).toBe('🟢 DE01 — CPU 12% · RAM 40%');
    expect(line({ ...base, phase: 'connected', d: d({ disk_used: 93 }) })).toMatch(/^🟠 .* 93%$/);
    expect(line({ ...base, phase: 'connecting', d: null })).toMatch(/^🟡 DE01 — /);
    expect(line({ ...base, phase: 'failed', d: null })).toMatch(/^🔴/);
    expect(line({ ...base, phase: null, d: null })).toMatch(/^⚪/);
  });

  it('drops server lines when locked and sorts by name', () => {
    expect(
      buildMenu([{ hostId: 'h', label: 'x', phase: 'connected', d: null }], true).items
    ).toEqual([]);
    const menu = buildMenu(
      [
        { hostId: 'b', label: 'NL', phase: null, d: null },
        { hostId: 'a', label: 'DE', phase: 'connected', d: null }
      ],
      false
    );
    expect(menu.items.map((i) => i.host_id)).toEqual(['a', 'b']);
    expect(buildMenu([], false).items[0]?.host_id).toBeNull();
  });
});
