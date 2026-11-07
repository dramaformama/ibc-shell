/** Folder/host tree: `$state.raw` + immutable updates, optimistic DnD with rollback. */
import {
  api,
  type Folder,
  type FolderDeleteMode,
  type FolderId,
  type FolderInput,
  type FolderPatch,
  type Host,
  type HostId,
  type HostInput,
  type HostPatch,
  type Result,
  type Tree
} from '$lib/api';
import { errorText } from '$lib/util/errors';
import {
  applyMoveLocal,
  buildRows,
  fkey,
  hkey,
  keyRef,
  type Drop,
  type NodeKey,
  type Row
} from '$lib/util/tree-model';
import { ui } from './ui.svelte';

class TreeStore {
  data = $state.raw<Tree>({ folders: [], hosts: [] });
  loaded = $state(false);
  selected = $state.raw<Set<NodeKey>>(new Set());
  cursor = $state<NodeKey | null>(null);
  rows = $derived<Row[]>(buildRows(this.data));

  host(id: HostId): Host | undefined {
    return this.data.hosts.find((h) => h.id === id);
  }
  folder(id: FolderId): Folder | undefined {
    return this.data.folders.find((f) => f.id === id);
  }

  /** Unwraps a result, toasting the error. */
  ok<T>(r: Result<T>): r is { status: 'ok'; data: T } {
    if (r.status === 'ok') return true;
    ui.toast(errorText(r.error), 'error');
    return false;
  }

  async load(): Promise<void> {
    const r = await api.treeList();
    if (this.ok(r)) {
      this.data = r.data;
      this.loaded = true;
      const live = new Set([
        ...r.data.folders.map((f) => fkey(f.id)),
        ...r.data.hosts.map((h) => hkey(h.id))
      ]);
      this.selected = new Set([...this.selected].filter((k) => live.has(k)));
    }
  }

  select(keys: Iterable<NodeKey>, cursor?: NodeKey): void {
    this.selected = new Set(keys);
    if (cursor) this.cursor = cursor;
  }

  /** Expands/collapses a folder: optimistic, persisted. */
  async setExpanded(id: FolderId, expanded: boolean): Promise<void> {
    const f = this.folder(id);
    if (!f || f.expanded === expanded) return;
    this.data = {
      ...this.data,
      folders: this.data.folders.map((x) => (x.id === id ? { ...x, expanded } : x))
    };
    const r = await api.folderUpdate(id, { expanded });
    if (!this.ok(r)) await this.load();
  }

  /** Moves nodes (in the given order) into one slot; optimistic with rollback. */
  async move(keys: NodeKey[], drop: Drop): Promise<boolean> {
    const snapshot = this.data;
    const moves: {
      ref: ReturnType<typeof keyRef>;
      parent: FolderId | null;
      after: ReturnType<typeof keyRef> | null;
    }[] = [];
    let after = drop.after;
    for (const k of keys) {
      const ref = keyRef(k);
      moves.push({ ref, parent: drop.parent, after });
      after = ref;
    }
    this.data = applyMoveLocal(this.data, moves);
    for (const m of moves) {
      const r =
        m.ref.kind === 'folder'
          ? await api.folderMove(m.ref.id, m.parent, m.after)
          : await api.hostMove(m.ref.id, m.parent, m.after);
      if (r.status === 'error') {
        this.data = snapshot;
        this.ok(r);
        await this.load();
        return false;
      }
    }
    if (drop.parent) void this.setExpanded(drop.parent, true);
    await this.load();
    return true;
  }

  async createHost(input: HostInput): Promise<Host | null> {
    const r = await api.hostCreate(input);
    if (!this.ok(r)) return null;
    await this.load();
    return r.data;
  }
  async updateHost(id: HostId, patch: HostPatch): Promise<boolean> {
    const r = await api.hostUpdate(id, patch);
    if (!this.ok(r)) return false;
    this.data = { ...this.data, hosts: this.data.hosts.map((h) => (h.id === id ? r.data : h)) };
    return true;
  }
  async createFolder(input: FolderInput): Promise<Folder | null> {
    const r = await api.folderCreate(input);
    if (!this.ok(r)) return null;
    await this.load();
    return r.data;
  }
  async updateFolder(id: FolderId, patch: FolderPatch): Promise<boolean> {
    const r = await api.folderUpdate(id, patch);
    if (!this.ok(r)) return false;
    await this.load();
    return true;
  }
  async duplicate(id: HostId): Promise<Host | null> {
    const r = await api.hostDuplicate(id);
    if (!this.ok(r)) return null;
    await this.load();
    return r.data;
  }
  async deleteHosts(ids: HostId[]): Promise<boolean> {
    const r = await api.hostDelete(ids);
    if (!this.ok(r)) return false;
    await this.load();
    return true;
  }
  async deleteFolder(id: FolderId, mode: FolderDeleteMode): Promise<boolean> {
    const r = await api.folderDelete(id, mode);
    if (!this.ok(r)) return false;
    await this.load();
    return true;
  }
  /** Sets (or clears with null) the flag of a host or folder. */
  async setFlag(key: NodeKey, code: string | null): Promise<boolean> {
    const ref = keyRef(key);
    return ref.kind === 'host'
      ? this.updateHost(ref.id, { country_code: code })
      : this.updateFolder(ref.id, { country_code: code });
  }
  async rename(key: NodeKey, name: string): Promise<boolean> {
    const ref = keyRef(key);
    const text = name.trim();
    if (!text) return false;
    return ref.kind === 'host'
      ? this.updateHost(ref.id, { label: text })
      : this.updateFolder(ref.id, { name: text });
  }
}

export const tree = new TreeStore();
