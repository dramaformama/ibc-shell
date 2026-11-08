/** Servers with a live terminal session, for the fleet pages. */
import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';

export interface LiveServer {
  key: string;
  sid: string;
  group: TabGroup;
}

export function connectedServers(): LiveServer[] {
  const seen = new Set<string>();
  return sessions.groups
    .map((group) => ({ key: group.key, sid: sessions.groupSession(group) ?? '', group }))
    .filter((s) => s.sid)
    .filter((s) => {
      // A server open in several windows is listed once.
      const id = s.group.current.hostId;
      if (!id) return true;
      if (seen.has(id)) return false;
      seen.add(id);
      return true;
    })
    .sort((a, b) => a.group.label.localeCompare(b.group.label));
}
