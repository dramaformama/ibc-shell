import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api, type HostWindowTransfer } from '$lib/api';
import { sessions, type TabKind } from '$lib/stores/sessions.svelte';
import { tree } from '$lib/stores/tree.svelte';
import { ui } from '$lib/stores/ui.svelte';
import * as m from '$lib/paraglide/messages.js';
import { transfers } from '$lib/sftp/transfers.svelte';
import { fileViews, releasedTabs } from '$lib/stores/window-transfer';
import { pauseTerminal, releaseTerminal, resumeTerminal } from '$lib/terminal/connection.svelte';
import { filesLink } from '$lib/terminal/files-link.svelte';
import { terminal } from '$lib/terminal/registry';
import { errorText } from '$lib/util/errors';

const native = isTauri();
export const isHostWindow = native
  ? getCurrentWindow().label.startsWith('host-')
  : hostWindowTarget() !== null;
export const canOpenHostWindow =
  !isHostWindow && (!native || !/Android|iPhone|iPad/.test(navigator.userAgent));

export function hostWindowTarget(): { hostId: string; kind: TabKind } | null {
  if (typeof location === 'undefined') return null;
  const params = new URLSearchParams(location.search);
  const hostId = native
    ? params.get('host')
    : (tree.data.hosts.find((h) => h.label === params.get('demoHost'))?.id ?? params.get('host'));
  const kind = params.get('view');
  return hostId && (kind === 'terminal' || kind === 'status' || kind === 'sftp')
    ? { hostId, kind }
    : null;
}

export async function openHostWindow(hostId: string, kind: TabKind = 'terminal') {
  try {
    const result = await api.hostWindowOpen(hostId, kind, null);
    if (result.status === 'ok') return true;
    ui.toast(errorText(result.error), 'error');
  } catch {
    ui.toast(m.err_generic(), 'error');
  }
  return false;
}

const moving = new Set<string>();

export async function detachGroup(key: string) {
  const group = sessions.groups.find((item) => item.key === key);
  const tab = group?.current;
  if (!canOpenHostWindow || !group || !tab?.hostId || moving.has(key)) return;
  moving.add(key);
  const terminalTab = group.tabs.find((item) => item.kind === 'terminal');
  const filesTab = group.tabs.find((item) => item.kind === 'sftp');
  const sid = terminalTab ? sessions.sessionIds[terminalTab.id] : null;
  const term = terminalTab ? terminal(terminalTab.id) : null;
  let transferred = false;

  try {
    const files = filesTab ? fileViews.get(filesTab.id)?.() : null;
    if (!native) {
      if (await openHostWindow(tab.hostId, tab.kind)) sessions.closeGroup(key);
      return;
    }
    if ((terminalTab && (!sid || !term)) || (filesTab && !files?.ready)) {
      ui.toast(m.host_window_wait_connection(), 'error');
      return;
    }
    if (sid && !(await pauseTerminal(sid))) throw new Error();
    const payload: HostWindowTransfer = {
      terminal: sid ?? null,
      files: files?.id ?? null,
      screen: (await term?.snapshot()) ?? null,
      cwd: terminalTab ? (filesLink.cwd[terminalTab.id] ?? null) : null,
      left: files?.left || null,
      right: files?.right || null,
      panel: ui.inspectorOpen,
      active_transfers: transfers.snapshot(),
      panel_tab: ui.panelTab
    };
    const opened = await api.hostWindowOpen(tab.hostId, tab.kind, payload);
    if (opened.status === 'error') {
      ui.toast(errorText(opened.error), 'error');
      return;
    }
    const label = opened.data;
    for (let tries = 0; tries < 100; tries++) {
      const result = await api.hostWindowTransferred(label, tries === 99);
      if (result.status === 'ok' && result.data) {
        transferred = true;
        if (terminalTab && sid) releasedTabs.add(terminalTab.id);
        if (filesTab && files?.id) releasedTabs.add(filesTab.id);
        sessions.closeGroup(key);
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 150));
    }
    ui.toast(m.err_generic(), 'error');
  } catch {
    ui.toast(m.err_generic(), 'error');
  } finally {
    if (sid) {
      if (transferred) releaseTerminal(sid);
      else void resumeTerminal(sid);
    }
    moving.delete(key);
  }
}
