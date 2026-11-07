/** Cross-store user actions shared by the tree, context menu, palette and shortcuts. */
import type { HostId, QuickConnect } from '$lib/api';
import { inheritedFlag } from '$lib/util/tree-model';
import { sessions } from './sessions.svelte';
import { tree } from './tree.svelte';

/** Opens a terminal (or SFTP) tab for a stored host. */
export function connectHost(hostId: HostId, kind: 'terminal' | 'sftp' = 'terminal'): void {
  const h = tree.host(hostId);
  if (!h) return;
  const flag = h.country_code ?? inheritedFlag(tree.data, h.folder_id)?.value ?? null;
  sessions.open({ kind, hostId, label: h.label, flag, adhoc: null });
}

/** Opens a terminal for an ad-hoc `user@host:port` target. */
export function connectAdhoc(target: QuickConnect): void {
  const label = `${target.user ? target.user + '@' : ''}${target.hostname}${target.port ? ':' + target.port : ''}`;
  sessions.open({ kind: 'terminal', hostId: null, label, flag: null, adhoc: target });
}
