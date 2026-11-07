/** The catalog defines the fleet; open views and TCP availability are independent observations. */
import type { Host, HostReachability, Tree } from '$lib/api';
import type { TabGroup, TabPhase } from '$lib/stores/sessions.svelte';
import { inheritedFlag } from '$lib/util/tree-model';

export type Availability = 'online' | 'offline' | 'unknown';
export interface Server {
  id: string;
  host: Host | null;
  group: TabGroup | null;
  label: string;
  address: string;
  flag: string | null;
  folder: string;
  availability: Availability;
  phase: TabPhase | null;
  latency: number | null;
}

/** Joins three sources without treating a closed view or authentication failure as an outage. */
export function buildFleet(
  catalog: Tree,
  groups: TabGroup[],
  phaseOf: (g: TabGroup) => TabPhase,
  observations: Record<string, HostReachability | null>,
  now: number
): Server[] {
  const folders = new Map(catalog.folders.map((f) => [f.id, f.name]));
  const byHost = new Map(groups.filter((g) => g.current.hostId).map((g) => [g.current.hostId, g]));
  const saved = catalog.hosts.map((host): Server => {
    const group = byHost.get(host.id) ?? null;
    const phase = group ? phaseOf(group) : null;
    const probe = observations[host.id];
    const fresh = probe && now - probe.checked_at <= 60_000;
    const availability: Availability =
      phase === 'connected'
        ? 'online'
        : fresh
          ? probe.reachable
            ? 'online'
            : 'offline'
          : 'unknown';
    return {
      id: host.id,
      host,
      group,
      phase,
      availability,
      label: host.label || host.hostname,
      address: host.hostname + (host.port && host.port !== 22 ? `:${host.port}` : ''),
      flag: host.country_code ?? inheritedFlag(catalog, host.folder_id)?.value ?? null,
      folder: host.folder_id ? (folders.get(host.folder_id) ?? '') : '',
      latency: fresh && probe.reachable ? probe.latency_ms : null
    };
  });
  const adhoc = groups
    .filter((g) => !g.current.hostId)
    .map((group): Server => ({
      id: group.key,
      host: null,
      group,
      phase: phaseOf(group),
      label: group.label,
      address: group.current.adhoc?.hostname ?? '',
      flag: group.flag,
      folder: '',
      availability: phaseOf(group) === 'connected' ? 'online' : 'unknown',
      latency: null
    }));
  return [...saved, ...adhoc];
}

/** Search includes folders and addresses, not only labels. */
export function filterFleet(
  servers: Server[],
  query: string,
  filter: 'all' | Availability
): Server[] {
  const q = query.trim().toLocaleLowerCase();
  return servers.filter(
    (s) =>
      (filter === 'all' || s.availability === filter) &&
      (!q || `${s.label} ${s.address} ${s.folder}`.toLocaleLowerCase().includes(q))
  );
}
