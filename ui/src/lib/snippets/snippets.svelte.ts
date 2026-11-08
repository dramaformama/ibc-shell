/**
 * Snippets: one library shared by every server. Bodies live in the store (`snippet_*`, always
 * global); folders, order and run counts in settings (`snippets.layout`).
 */
import { api, type Snippet } from '$lib/api';
import * as m from '$lib/paraglide/messages.js';
import { connectInBackground } from '$lib/stores/actions';
import { sessions } from '$lib/stores/sessions.svelte';
import { ui } from '$lib/stores/ui.svelte';
import { errorText } from '$lib/util/errors';
import { terminal, whenConnected } from '$lib/terminal/registry';
import { emptyLayout, keystrokes, moveSnippet, parseVars, readLayout, type Layout } from './model';

const KEY = 'snippets.layout';

/** A run waiting for variable values and/or target servers (RunDialog). */
export interface PendingRun {
  snippet: Snippet;
  /** `here` = the current server's terminal; `many` = pick servers. */
  mode: 'here' | 'many';
  execute: boolean;
}

/** A server a snippet can run on: an open terminal tab or a saved host to connect. */
export interface Target {
  tabId: string | null;
  hostId: string | null;
}

/** Pause between the typed text and the Enter that runs it. */
const ENTER_DELAY_MS = 80;

class SnippetStore {
  items = $state.raw<Snippet[]>([]);
  layout = $state.raw<Layout>(emptyLayout());
  loaded = $state(false);
  pending = $state<PendingRun | null>(null);
  /** Editor sheet: a snippet to edit, `new` (optional folder), or closed. */
  editing = $state<{ snippet: Snippet | null; group: string | null } | null>(null);
  /** Folder name dialog: `id` null = create, else rename. */
  folderDialog = $state<{ id: string | null } | null>(null);
  /** Folder awaiting delete confirmation. */
  folderDelete = $state<string | null>(null);
  /** Folder card picked in the panel (null = show everything). */
  selectedGroup = $state<string | null>(null);

  async load(): Promise<void> {
    const [list, lay] = await Promise.all([api.snippetList(null), api.settingsGet(KEY)]);
    if (list.status === 'ok') this.items = list.data;
    if (lay.status === 'ok') this.layout = readLayout(lay.data);
    this.loaded = true;
  }

  clear(): void {
    this.items = [];
    this.layout = emptyLayout();
    this.pending = null;
    this.editing = null;
    this.folderDialog = null;
    this.folderDelete = null;
    this.selectedGroup = null;
    this.loaded = false;
  }

  #save(l: Layout): void {
    this.layout = l;
    void api.settingsSet(KEY, l as unknown as Parameters<typeof api.settingsSet>[1]);
  }

  async save(name: string, body: string, group: string | null, id?: string): Promise<boolean> {
    const r = id
      ? await api.snippetUpdate(id, { name, body })
      : await api.snippetCreate({ name, body, folder_id: null });
    if (r.status === 'error') {
      ui.toast(errorText(r.error), 'error');
      return false;
    }
    const s = r.data;
    this.items = id ? this.items.map((x) => (x.id === id ? s : x)) : [...this.items, s];
    if ((this.layout.of[s.id] ?? null) !== group || !id)
      this.#save(moveSnippet(this.layout, this.#ids(), s.id, group, null));
    return true;
  }

  async remove(id: string): Promise<void> {
    const r = await api.snippetDelete(id);
    if (r.status === 'error') return ui.toast(errorText(r.error), 'error');
    this.items = this.items.filter((x) => x.id !== id);
    const of = { ...this.layout.of };
    delete of[id];
    const uses = { ...this.layout.uses };
    delete uses[id];
    this.#save({ ...this.layout, of, uses, order: this.layout.order.filter((x) => x !== id) });
  }

  #ids(): string[] {
    return this.items.map((x) => x.id);
  }

  move(id: string, group: string | null, before: string | null): void {
    if (id === before) return;
    this.#save(moveSnippet(this.layout, this.#ids(), id, group, before));
  }

  addGroup(name: string): string {
    const id = `g${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    this.#save({ ...this.layout, groups: [...this.layout.groups, { id, name }] });
    return id;
  }

  renameGroup(id: string, name: string): void {
    this.#save({
      ...this.layout,
      groups: this.layout.groups.map((g) => (g.id === id ? { ...g, name } : g))
    });
  }

  /** Deletes a folder; its snippets move to "No folder". */
  removeGroup(id: string): void {
    if (this.selectedGroup === id) this.selectedGroup = null;
    const of = Object.fromEntries(Object.entries(this.layout.of).filter(([, g]) => g !== id));
    this.#save({ ...this.layout, groups: this.layout.groups.filter((g) => g.id !== id), of });
  }

  moveGroup(id: string, delta: -1 | 1): void {
    const gs = [...this.layout.groups];
    const i = gs.findIndex((g) => g.id === id);
    const j = i + delta;
    if (i < 0 || j < 0 || j >= gs.length) return;
    [gs[i], gs[j]] = [gs[j]!, gs[i]!];
    this.#save({ ...this.layout, groups: gs });
  }

  toggleGroup(id: string): void {
    const c = this.layout.collapsed;
    this.#save({
      ...this.layout,
      collapsed: c.includes(id) ? c.filter((x) => x !== id) : [...c, id]
    });
  }

  /** Entry point for a click: asks for variables / servers when needed, else types at once. */
  run(snippet: Snippet, mode: 'here' | 'many' = 'here', execute = true): void {
    if (mode === 'here' && !this.currentTarget().hostId) mode = 'many';
    if (mode === 'many' || parseVars(snippet.body).length) {
      this.pending = { snippet, mode, execute };
      return;
    }
    void this.send(snippet, snippet.body, [this.currentTarget()], execute);
  }

  /** The active server's terminal (opened behind the current view when missing). */
  currentTarget(): Target {
    const g = sessions.active ? sessions.groupOf(sessions.active.id) : null;
    const t = g?.tabs.find((x) => x.kind === 'terminal');
    return { tabId: t?.id ?? null, hostId: g?.current.hostId ?? null };
  }

  /** Types `body` into every target, connecting saved hosts that have no terminal yet. */
  async send(snippet: Snippet, body: string, targets: Target[], execute: boolean): Promise<void> {
    this.#save({
      ...this.layout,
      uses: { ...this.layout.uses, [snippet.id]: (this.layout.uses[snippet.id] ?? 0) + 1 }
    });
    // Enter goes as its own keystroke a moment after the text, exactly like pressing it by hand.
    // In one chunk with the text some shells and scripts swallow it, and the command only sat
    // on the prompt.
    const text = keystrokes(body, false);
    const tabs = targets
      .map((t) => {
        if (t.tabId) return t.tabId;
        if (!t.hostId) return null;
        connectInBackground(t.hostId);
        return sessions.find(t.hostId, 'terminal')?.id ?? null;
      })
      .filter((x): x is string => !!x);
    const results = await Promise.all(
      tabs.map(async (id) => {
        const h = terminal(id)?.phase() === 'connected' ? terminal(id) : await whenConnected(id);
        h?.send(text);
        if (h && execute) setTimeout(() => h.send('\r'), ENTER_DELAY_MS);
        return !!h;
      })
    );
    const ok = results.filter(Boolean).length;
    if (tabs.length === 1 && ok === 1) {
      const active = sessions.active;
      if (active && tabs[0] === active.id) terminal(active.id)?.focus();
    }
    if (tabs.length > 1 || ok < tabs.length)
      ui.toast(
        m.snip_sent({ ok, total: tabs.length }),
        ok === tabs.length ? 'info' : ok ? 'info' : 'error'
      );
  }
}

export const snippets = new SnippetStore();
