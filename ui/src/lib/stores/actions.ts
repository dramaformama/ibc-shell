/** Cross-store user actions shared by the tree, context menu, palette and shortcuts. */
import type { HostId, QuickConnect } from '$lib/api';
import { inheritedFlag } from '$lib/util/tree-model';
import { sessions } from './sessions.svelte';
import { tree } from './tree.svelte';

/** Opens a terminal (or SFTP) tab for a stored host. */
export function connectHost(hostId: HostId, kind: 'terminal' | 'sftp' = 'terminal'): void {
  const h = tree.host(hostId);
  if (!h) return;
  const open = sessions.find(hostId, kind);
  if (open) return sessions.select(open.id);
  const flag = h.country_code ?? inheritedFlag(tree.data, h.folder_id)?.value ?? null;
  sessions.open({ kind, hostId, label: h.label, flag, adhoc: null });
}

/** Opens a terminal for an ad-hoc `user@host:port` target. */
export function connectAdhoc(target: QuickConnect): void {
  const label = `${target.user ? target.user + '@' : ''}${target.hostname}${target.port ? ':' + target.port : ''}`;
  sessions.open({ kind: 'terminal', hostId: null, label, flag: null, adhoc: target });
}

/** Switches the active host tab between terminal and files, reusing an open tab when there is one. */
export function switchKind(kind: 'terminal' | 'sftp'): void {
  const t = sessions.active;
  if (!t?.hostId || t.kind === kind) return;
  const open = sessions.find(t.hostId, kind);
  if (open) sessions.select(open.id);
  else connectHost(t.hostId, kind);
}
