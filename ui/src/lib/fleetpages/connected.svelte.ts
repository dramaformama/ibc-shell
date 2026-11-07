/** Servers with a live terminal session, for the fleet pages. */
import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';

export interface LiveServer {
  key: string;
  sid: string;
  group: TabGroup;
}

export function connectedServers(): LiveServer[] {
  return sessions.groups
    .map((group) => ({ key: group.key, sid: sessions.groupSession(group) ?? '', group }))
    .filter((s) => s.sid)
    .sort((a, b) => a.group.label.localeCompare(b.group.label));
}
