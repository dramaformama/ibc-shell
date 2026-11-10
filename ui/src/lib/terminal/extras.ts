/**
 * Terminal add-ons that live outside xterm: files dropped from the OS onto a terminal, and
 * telling the user that a command in a background terminal finished.
 */
import { api } from '$lib/api';
import * as m from '$lib/paraglide/messages.js';
import type { Tab } from '$lib/stores/sessions.svelte';
import { ui } from '$lib/stores/ui.svelte';
import type { Notice } from './activity';
import type { Terminal } from '@xterm/xterm';
import { cwdFromTitle, parseOsc7, shellQuote } from './cwd';
import { filesLink } from './files-link.svelte';
import { terminalPrefs } from './prefs.svelte';

export interface DropTarget {
  /** The pane element; drops elsewhere are not ours. */
  el: () => HTMLElement | null;
  /** Only the visible terminal takes drops. */
  active: () => boolean;
  /** Connected and able to take input / uploads. */
  ready: () => boolean;
  /** Hover state for the overlay (`null` = not over this terminal). */
  over: (on: boolean) => void;
  /** Local terminal: the paths are typed in, like Terminal.app does. */
  paste: (text: string) => void;
}

/**
 * Listens to OS file drops (Tauri gives local paths and a physical-pixel position). On a
 * server terminal the files are uploaded into the shell's folder; on the local one, typed in.
 */
export async function listenDrop(tab: Tab, t: DropTarget): Promise<() => void> {
  try {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    return await getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === 'leave') return t.over(false);
      if (!t.active()) return;
      const dpr = window.devicePixelRatio || 1;
      const hit = document.elementFromPoint(p.position.x / dpr, p.position.y / dpr);
      const inside = !!hit && !!t.el()?.contains(hit) && t.ready();
      if (p.type !== 'drop') return t.over(inside);
      t.over(false);
      if (!inside || !p.paths.length) return;
      if (tab.local) return t.paste(p.paths.map(shellQuote).join(' ') + ' ');
      if (tab.hostId) void upload(tab, p.paths);
    });
  } catch {
    return () => {};
  }
}

async function upload(tab: Tab, paths: string[]): Promise<void> {
  const dir = await filesLink.upload(tab, paths);
  if (dir) ui.toast(m.term_drop_started({ count: paths.length, dir }));
  else ui.toast(m.sftp_enqueue_failed(), 'error');
}

/** In the app: a toast; app in the background: a system notification. */
export function tell(tab: Tab, notice: Notice): void {
  if (!notice || !terminalPrefs.notify) return;
  const body = notice === 'bell' ? m.notify_bell() : m.notify_done();
  if (document.hasFocus()) ui.toast(`${tab.label}: ${body}`);
  else void api.appNotify(tab.label, body);
}

/** Follows the shell's folder: OSC 7 when the shell reports it, else the Debian-style title. */
export function trackFolder(term: Terminal, tabId: string): void {
  let osc7 = false;
  term.onTitleChange((t) => {
    const dir = osc7 ? null : cwdFromTitle(t);
    if (dir) filesLink.set(tabId, dir);
  });
  term.parser.registerOscHandler(7, (data) => {
    const dir = parseOsc7(data);
    if (dir) {
      osc7 = true;
      filesLink.set(tabId, dir);
    }
    return true;
  });
}
