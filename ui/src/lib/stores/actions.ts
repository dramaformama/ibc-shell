/** Cross-store user actions shared by the tree, context menu, palette and shortcuts. */
import type { HostId, QuickConnect } from '$lib/api';
import { inheritedFlag } from '$lib/util/tree-model';
import { recent } from './recent.svelte';
import { sessions, type TabKind } from './sessions.svelte';
import { tree } from './tree.svelte';
import { ui } from './ui.svelte';

function openFor(hostId: HostId, kind: TabKind, focus: boolean, instance = 1): void {
  const h = tree.host(hostId);
  if (!h) return;
  const flag = h.country_code ?? inheritedFlag(tree.data, h.folder_id)?.value ?? null;
  const label = instance > 1 ? `${h.label} · ${instance}` : h.label;
  sessions.open({ kind, hostId, label, flag, adhoc: null, instance }, focus);
}

/**
 * Opens a view of a stored host, reusing an open one. Status reads metrics over the server's
 * terminal session, so a terminal is started behind it when none is open.
 */
export function connectHost(hostId: HostId, kind: TabKind = 'terminal', instance = 1): void {
  if (!tree.host(hostId)) return;
  recent.push(hostId);
  const open = sessions.find(hostId, kind, instance);
  if (open) return sessions.select(open.id);
  if (kind === 'status' && !sessions.find(hostId, 'terminal', instance))
    openFor(hostId, 'terminal', false, instance);
  openFor(hostId, kind, true, instance);
}

/** Opens one more independent window (its own connection) of a saved host. */
export function connectAnother(hostId: HostId): void {
  connectHost(hostId, 'terminal', sessions.nextInstance(hostId));
}

/** Starts a terminal without switching to it (group open, run-everywhere). */
export function connectInBackground(hostId: HostId): void {
  if (sessions.find(hostId, 'terminal')) return;
  recent.push(hostId);
  openFor(hostId, 'terminal', false);
}

/** Every saved host inside a folder, including subfolders, in tree order. */
export function hostsInFolder(folderId: string): HostId[] {
  const inside = new Set<string>([folderId]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const f of tree.data.folders)
      if (f.parent_id && inside.has(f.parent_id) && !inside.has(f.id)) {
        inside.add(f.id);
        grew = true;
      }
  }
  return tree.data.hosts.filter((h) => h.folder_id && inside.has(h.folder_id)).map((h) => h.id);
}

/** Opens a terminal for every host of a folder; the first one is shown. */
export function connectFolder(folderId: string): void {
  const ids = hostsInFolder(folderId);
  ids.forEach((id, i) => (i === 0 ? connectHost(id) : connectInBackground(id)));
}

/** Opens a terminal for an ad-hoc `user@host:port` target. */
export function connectAdhoc(target: QuickConnect): void {
  const label = `${target.user ? target.user + '@' : ''}${target.hostname}${target.port ? ':' + target.port : ''}`;
  sessions.open({ kind: 'terminal', hostId: null, label, flag: null, adhoc: target });
}

/** Switches the active server between status, terminal and files, reusing open views. */
export function switchKind(kind: TabKind): void {
  const t = sessions.active;
  if (!t?.hostId || t.kind === kind) return;
  const open = sessions.find(t.hostId, kind, t.instance ?? 1);
  if (open) sessions.select(open.id);
  else connectHost(t.hostId, kind, t.instance ?? 1);
}

/**
 * The user closes a server tab. A saved server that is live keeps feeding the overview, so
 * closing asks whether to keep it in the background for monitoring or to disconnect.
 */
export function closeServer(key: string): void {
  const g = sessions.groups.find((x) => x.key === key);
  const live = !!g && !!g.tabs[0]?.hostId && sessions.groupPhase(g) === 'connected';
  if (live) ui.closeAsk = key;
  else sessions.closeGroup(key);
}
