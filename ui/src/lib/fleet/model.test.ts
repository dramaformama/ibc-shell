import { describe, expect, it } from 'vitest';
import { db } from '$lib/api/mock/state';
import type { HostReachability, Tree } from '$lib/api';
import type { Tab, TabGroup } from '$lib/stores/sessions.svelte';
import { buildFleet, filterFleet } from './model';

const catalog: Tree = { hosts: db.hosts, folders: db.folders };
const host = catalog.hosts[0]!;
const tab: Tab = {
  id: 'terminal',
  kind: 'terminal',
  hostId: host.id,
  label: host.label,
  flag: null,
  adhoc: null
};
const group: TabGroup = {
  key: `h:${host.id}`,
  label: host.label,
  flag: null,
  current: tab,
  tabs: [tab]
};
const now = 1_000_000;
const observation = (reachable: boolean, checked_at = now): Record<string, HostReachability> => ({
  [host.id]: { reachable, checked_at, latency_ms: reachable ? 24 : null }
});

describe('server-first fleet', () => {
  it('shows saved hosts with no open tabs, without inventing an outage', () => {
    const servers = buildFleet(catalog, [], () => 'disconnected', {}, now);
    expect(servers).toHaveLength(catalog.hosts.length);
    expect(servers.every((s) => s.availability === 'unknown')).toBe(true);
  });
  it('keeps reachability distinct from failed authentication', () => {
    const server = buildFleet(catalog, [group], () => 'failed', observation(true), now)[0]!;
    expect(server.phase).toBe('failed');
    expect(server.availability).toBe('online');
    expect(server.latency).toBe(24);
  });
  it('uses live connections over old probes, and expires old observations', () => {
    expect(
      buildFleet(catalog, [group], () => 'connected', observation(false), now)[0]?.availability
    ).toBe('online');
    expect(
      buildFleet(catalog, [], () => 'disconnected', observation(false, 1), now)[0]?.availability
    ).toBe('unknown');
    expect(
      buildFleet(catalog, [], () => 'disconnected', observation(false), now)[0]?.availability
    ).toBe('offline');
  });
  it('does not duplicate saved hosts with multiple views; resolves folder flag and filters', () => {
    const servers = buildFleet(catalog, [group], () => 'connected', {}, now);
    expect(servers).toHaveLength(catalog.hosts.length);
    expect(servers[0]?.flag).toBe('de');
    expect(filterFleet(servers, host.hostname, 'online').map((s) => s.id)).toEqual([host.id]);
    expect(filterFleet(servers, 'germany', 'all').length).toBeGreaterThan(0);
    expect(filterFleet(servers, 'missing-server', 'all')).toEqual([]);
  });
});
