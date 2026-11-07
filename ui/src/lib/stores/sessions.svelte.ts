/**
 * Open tabs. The shell owns only the tab list and selection; terminal / SFTP content is rendered
 * by their own routes (terminal-ui / sftp-ui) which attach a real session to a tab.
 */
import type { HostId, QuickConnect } from '$lib/api';

export type TabKind = 'terminal' | 'sftp';

export interface Tab {
  id: string;
  kind: TabKind;
  hostId: HostId | null;
  label: string;
  /** Flag code already resolved for display, or null. */
  flag: string | null;
  adhoc: QuickConnect | null;
}

class SessionsStore {
  tabs = $state.raw<Tab[]>([]);
  activeId = $state<string | null>(null);
  #seq = 0;

  get active(): Tab | null {
    return this.tabs.find((t) => t.id === this.activeId) ?? null;
  }

  open(tab: Omit<Tab, 'id'>): Tab {
    const t: Tab = { ...tab, id: `tab-${++this.#seq}` };
    this.tabs = [...this.tabs, t];
    this.activeId = t.id;
    return t;
  }

  close(id: string): void {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i === -1) return;
    this.tabs = this.tabs.filter((t) => t.id !== id);
    if (this.activeId === id)
      this.activeId = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null;
  }

  /** ⌘1…9 / ⌃Tab navigation. */
  step(delta: 1 | -1): void {
    if (this.tabs.length < 2) return;
    const i = this.tabs.findIndex((t) => t.id === this.activeId);
    this.activeId = this.tabs[(i + delta + this.tabs.length) % this.tabs.length]?.id ?? null;
  }

  hasHost(hostId: HostId): boolean {
    return this.tabs.some((t) => t.hostId === hostId);
  }

  clear(): void {
    this.tabs = [];
    this.activeId = null;
  }
}

export const sessions = new SessionsStore();
