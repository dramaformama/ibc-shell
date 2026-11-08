/**
 * Open tabs. The shell owns only the tab list and selection; terminal / SFTP content is rendered
 * by their own routes (terminal-ui / sftp-ui) which attach a real session to a tab.
 */
import type { HostId, QuickConnect } from '$lib/api';

/** `status` is the server's metrics page; it has no connection of its own (it reads the terminal's). */
export type TabKind = 'terminal' | 'sftp' | 'status';

/** Connection state shown next to a tab (sidebar dot, tab dot). */
export type TabPhase = 'connecting' | 'connected' | 'failed' | 'disconnected';

export interface Tab {
  id: string;
  kind: TabKind;
  hostId: HostId | null;
  label: string;
  /** Flag code already resolved for display, or null. */
  flag: string | null;
  adhoc: QuickConnect | null;
  /** Opened for monitoring (startup, group connect): live, but not shown in the tab bar until the user opens it. */
  background?: boolean;
}

/** One server in the tab bar / sidebar: its terminal and files views share a group. */
export interface TabGroup {
  key: string;
  label: string;
  flag: string | null;
  tabs: Tab[];
  /** The member shown when the group is selected. */
  current: Tab;
}

/** Tabs of one stored host form a group; ad-hoc tabs stand alone. */
export const groupKey = (t: Tab): string => (t.hostId ? `h:${t.hostId}` : t.id);

class SessionsStore {
  tabs = $state.raw<Tab[]>([]);
  activeId = $state<string | null>(null);
  /** Per-tab connection state, reported by the terminal / SFTP views. */
  phases = $state<Record<string, TabPhase>>({});
  /** Backend session id per terminal tab, once connected (stats use it). */
  sessionIds = $state<Record<string, string>>({});
  /** Last active tab per group, so switching servers returns to the same view. */
  #last = new Map<string, string>();
  #seq = 0;

  get groups(): TabGroup[] {
    const out = new Map<string, Tab[]>();
    for (const t of this.tabs) {
      const k = groupKey(t);
      out.set(k, [...(out.get(k) ?? []), t]);
    }
    return [...out].map(([key, tabs]) => {
      const first = tabs[0]!;
      const current =
        tabs.find((t) => t.id === this.activeId) ??
        tabs.find((t) => t.id === this.#last.get(key)) ??
        first;
      return { key, label: first.label, flag: first.flag, tabs, current };
    });
  }

  /** Servers with at least one tab the user has opened; the rest only feed the overview. */
  get visibleGroups(): TabGroup[] {
    return this.groups.filter((g) => g.tabs.some((t) => !t.background));
  }

  get activeGroup(): string | null {
    return this.active ? groupKey(this.active) : null;
  }

  /** One dot for a server: connected wins, then connecting, then the current view's state. */
  groupPhase(g: TabGroup): TabPhase {
    const views = g.tabs.filter((t) => t.kind !== 'status');
    const ps = views.map((t) => this.phase(t.id));
    if (ps.includes('connected')) return 'connected';
    if (ps.includes('connecting')) return 'connecting';
    const cur = views.find((t) => t.id === g.current.id) ?? views[0];
    return cur ? this.phase(cur.id) : 'disconnected';
  }

  /** Selects a group (its last used view). */
  activate(key: string): void {
    const g = this.groups.find((x) => x.key === key);
    if (g) this.select(g.current.id);
  }

  select(id: string): void {
    const t = this.tabs.find((x) => x.id === id);
    if (!t) return;
    // Opening a background server is what makes it a tab.
    if (t.background)
      this.tabs = this.tabs.map((x) => (x.id === id ? { ...x, background: false } : x));
    this.#last.set(groupKey(t), id);
    this.activeId = id;
  }

  /** Closes every view of a server (disconnect). */
  closeGroup(key: string): void {
    for (const t of this.tabs.filter((x) => groupKey(x) === key)) this.close(t.id);
  }

  phase(id: string): TabPhase {
    return this.phases[id] ?? 'connecting';
  }

  setSessionId(tabId: string, sid: string | null): void {
    if (sid) this.sessionIds[tabId] = sid;
    else delete this.sessionIds[tabId];
  }

  /** The group a tab belongs to. */
  groupOf(tabId: string): TabGroup | null {
    return this.groups.find((g) => g.tabs.some((t) => t.id === tabId)) ?? null;
  }

  /** Session id of a server group's terminal (metrics run over it). */
  groupSession(g: TabGroup): string | null {
    const t = g.tabs.find((x) => x.kind === 'terminal' && this.sessionIds[x.id]);
    return t ? (this.sessionIds[t.id] ?? null) : null;
  }

  setPhase(id: string, phase: TabPhase): void {
    if (!this.tabs.some((t) => t.id === id)) return;
    if (this.phases[id] !== phase) this.phases[id] = phase;
  }

  /** The tab of `kind` already open for `hostId`, if any. */
  find(hostId: HostId, kind: TabKind): Tab | null {
    return this.tabs.find((t) => t.hostId === hostId && t.kind === kind) ?? null;
  }

  get active(): Tab | null {
    return this.tabs.find((t) => t.id === this.activeId) ?? null;
  }

  /** Adds a tab; `focus = false` opens it in the background (e.g. a terminal behind Status). */
  open(tab: Omit<Tab, 'id'>, focus = true): Tab {
    const t: Tab = { ...tab, id: `tab-${++this.#seq}`, background: !focus };
    this.tabs = [...this.tabs, t];
    if (focus) this.select(t.id);
    return t;
  }

  close(id: string): void {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i === -1) return;
    const closing = this.tabs[i]!;
    this.tabs = this.tabs.filter((t) => t.id !== id);
    delete this.phases[id];
    delete this.sessionIds[id];
    if (this.activeId === id) {
      // Prefer the other view of the same server, then the neighbour.
      const shown = this.tabs.filter((t) => !t.background);
      const sibling = shown.find((t) => groupKey(t) === groupKey(closing));
      const near = shown.length ? shown[Math.min(i, shown.length - 1)] : undefined;
      this.activeId = sibling?.id ?? near?.id ?? null;
    }
  }

  /** ⌘1…9 / ⌃Tab navigation over servers. */
  step(delta: 1 | -1): void {
    const gs = this.visibleGroups;
    if (gs.length < 2) return;
    const i = gs.findIndex((g) => g.key === this.activeGroup);
    const g = gs[(i + delta + gs.length) % gs.length];
    if (g) this.select(g.current.id);
  }

  hasHost(hostId: HostId): boolean {
    return this.tabs.some((t) => t.hostId === hostId);
  }

  clear(): void {
    this.tabs = [];
    this.phases = {};
    this.sessionIds = {};
    this.#last.clear();
    this.activeId = null;
  }
}

export const sessions = new SessionsStore();
