import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api, apiEvents, type HostWindowTransfer, type Result } from '$lib/api';
import { connectHost } from '$lib/stores/actions';
import { groupKey, sessions, type Tab, type TabKind } from '$lib/stores/sessions.svelte';
import { tree } from '$lib/stores/tree.svelte';
import { ui } from '$lib/stores/ui.svelte';
import * as m from '$lib/paraglide/messages.js';
import { transfers } from '$lib/sftp/transfers.svelte';
import { adoptWindowReturn, fileViews, releasedTabs } from '$lib/stores/window-transfer';
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

type Send = (hostId: string, kind: TabKind, payload: HostWindowTransfer) => Promise<Result<string>>;

/**
 * Hands a tab group's live sessions to another window: pauses the terminal output, sends the
 * payload, waits until the destination took it and then lets `done` drop the local tab.
 */
async function handOff(key: string, send: Send, done: () => void) {
  const group = sessions.groups.find((item) => item.key === key);
  const tab = group?.current;
  if (!group || !tab?.hostId || moving.has(key)) return;
  moving.add(key);
  const terminalTab = group.tabs.find((item) => item.kind === 'terminal');
  const filesTab = group.tabs.find((item) => item.kind === 'sftp');
  const sid = terminalTab ? sessions.sessionIds[terminalTab.id] : null;
  const term = terminalTab ? terminal(terminalTab.id) : null;
  let transferred = false;

  try {
    const files = filesTab ? fileViews.get(filesTab.id)?.() : null;
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
    const opened = await send(tab.hostId, tab.kind, payload);
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
        done();
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

/** Main window: moves a tab group into its own window. */
export async function detachGroup(key: string) {
  const tab = sessions.groups.find((item) => item.key === key)?.current;
  if (!canOpenHostWindow || !tab?.hostId) return;
  if (!native) {
    if (await openHostWindow(tab.hostId, tab.kind)) sessions.closeGroup(key);
    return;
  }
  await handOff(
    key,
    (hostId, kind, payload) => api.hostWindowOpen(hostId, kind, payload),
    () => sessions.closeGroup(key)
  );
}

/** Host windows can give their tab back to the main window (not in the browser preview). */
export const canReturnToMain = native && isHostWindow;

/** Host window: puts its tab back into the main window's tab bar and closes this window. */
export async function returnToMain(key: string) {
  if (!canReturnToMain) return;
  await handOff(
    key,
    (hostId, kind, payload) => api.hostWindowReturn(hostId, kind, payload),
    () => void getCurrentWindow().close()
  );
}

/** Main window: opens tabs for sessions a host window returned (`host-window://return`). */
export function listenWindowReturns() {
  return apiEvents.listen('hostWindowReturn', async ({ label, host_id, view }) => {
    // The host may already be open here: the returned connection becomes one more instance.
    const instance = sessions.nextInstance(host_id);
    const key = groupKey({ hostId: host_id, instance } as Tab);
    const received = await adoptWindowReturn(label, key);
    if (received.status !== 'ok' || !received.data) {
      ui.toast(received.status === 'error' ? errorText(received.error) : m.err_generic(), 'error');
      return;
    }
    if (received.data.terminal && view !== 'terminal') connectHost(host_id, 'terminal', instance);
    if (received.data.files && view !== 'sftp') connectHost(host_id, 'sftp', instance);
    connectHost(host_id, view, instance);
  });
}
