/**
 * Terminal ↔ Files: remembers each terminal's folder, opens it in the server's Files view, and
 * uploads files dropped onto a terminal into that folder (over the terminal's own connection).
 */
import { Channel } from '@tauri-apps/api/core';
import { api, type SessionEvent, type SftpId } from '$lib/api';
import { switchKind } from '$lib/stores/actions';
import { groupKey, sessions, type Tab } from '$lib/stores/sessions.svelte';
import { transfers } from '$lib/sftp/transfers.svelte';
import { expandHome } from './cwd';

function channel(): Channel<SessionEvent> {
  try {
    return new Channel<SessionEvent>();
  } catch {
    return { onmessage: () => {} } as unknown as Channel<SessionEvent>;
  }
}

class FilesLink {
  /** Last folder reported by each terminal tab (may start with `~`). */
  cwd = $state<Record<string, string>>({});
  /** Folder the Files view of a server should show next (set by "Open in Files"). */
  request = $state<Record<string, string>>({});
  /** SFTP sessions opened for drops, one per terminal tab. */
  #sftp = new Map<string, Promise<SftpId | null>>();

  set(tabId: string, path: string): void {
    if (this.cwd[tabId] !== path) this.cwd[tabId] = path;
  }

  /** Switches the server to Files at the terminal's folder. */
  openInFiles(tab: Tab): void {
    const path = this.cwd[tab.id];
    if (!tab.hostId || !path) return;
    this.request[groupKey(tab)] = path;
    if (sessions.activeId !== tab.id) sessions.select(tab.id);
    switchKind('sftp');
  }

  /** The pending folder for a server's Files view, consumed once. */
  take(key: string): string | null {
    const p = this.request[key];
    if (p === undefined) return null;
    delete this.request[key];
    return p;
  }

  #open(tab: Tab): Promise<SftpId | null> {
    let p = this.#sftp.get(tab.id);
    if (!p) {
      const sid = sessions.sessionIds[tab.id];
      if (!sid) return Promise.resolve(null);
      p = api
        .sftpOpen({ kind: 'session', session_id: sid }, channel())
        .then((r) => (r.status === 'ok' ? r.data : null));
      p.then((id) => id ?? this.#sftp.delete(tab.id));
      this.#sftp.set(tab.id, p);
    }
    return p;
  }

  /** Absolute target folder for a drop: the terminal's folder, or the home folder. */
  async target(tab: Tab, sftp: SftpId): Promise<string | null> {
    const cwd = this.cwd[tab.id] ?? '~';
    if (!cwd.startsWith('~')) return cwd;
    const home = await api.sftpRealpath(sftp, '.');
    return home.status === 'ok' ? expandHome(cwd, home.data) : null;
  }

  /**
   * Uploads local files into the terminal's folder. Never overwrites: a name that exists gets
   * ` (1)` (there is no conflict dialog outside Files). Returns the folder, or null on failure.
   */
  async upload(tab: Tab, paths: string[]): Promise<string | null> {
    const sftp = await this.#open(tab);
    if (!sftp) return null;
    const dir = await this.target(tab, sftp);
    if (!dir) return null;
    const ok = await transfers.enqueue(
      { kind: 'local' },
      { kind: 'remote', sftp_id: sftp },
      paths,
      dir,
      'Rename'
    );
    return ok ? dir : null;
  }

  /** The terminal tab closed: forget its folder and close its drop session. */
  release(tabId: string): void {
    delete this.cwd[tabId];
    const p = this.#sftp.get(tabId);
    this.#sftp.delete(tabId);
    void p?.then((id) => {
      if (id) void api.sftpClose(id);
    });
  }
}

export const filesLink = new FilesLink();
