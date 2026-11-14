/** One SFTP pane: endpoint + path + listing + selection + sort + filter + history. */
import { api, type ApiError, type DirListing, type Endpoint, type Entry } from '$lib/api';
import {
  emptySelection,
  filterEntries,
  joinPath,
  parentPath,
  prune,
  select,
  sortEntries,
  type Selection,
  type Sort
} from './model';

type Res<T> = { status: 'ok'; data: T } | { status: 'error'; error: ApiError };

/** Endpoint-agnostic filesystem calls. */
export function fsOps(ep: Endpoint) {
  const remote = ep.kind === 'remote' ? ep.sftp_id : null;
  return {
    home: (): Promise<Res<string>> => (remote ? api.sftpRealpath(remote, '.') : api.localHome()),
    list: (p: string): Promise<Res<DirListing>> =>
      remote ? api.sftpList(remote, p) : api.localList(p),
    mkdir: (p: string): Promise<Res<null>> =>
      remote ? api.sftpMkdir(remote, p) : api.localMkdir(p),
    rename: (a: string, b: string): Promise<Res<null>> =>
      remote ? api.sftpRename(remote, a, b) : api.localRename(a, b),
    remove: (p: string): Promise<Res<null>> =>
      remote ? api.sftpRemove(remote, p, true) : api.localRemove(p, true),
    chmod: (p: string, mode: number): Promise<Res<null>> | null =>
      remote ? api.sftpChmod(remote, p, mode) : null
  };
}

export class Pane {
  endpoint = $state.raw<Endpoint>({ kind: 'local' });
  path = $state('');
  entries = $state.raw<Entry[]>([]);
  selection = $state.raw<Selection>(emptySelection);
  sort = $state.raw<Sort>({ key: 'name', dir: 'asc' });
  filter = $state('');
  showHidden = $state(false);
  loading = $state(false);
  error = $state<ApiError | null>(null);
  back = $state.raw<string[]>([]);
  forward = $state.raw<string[]>([]);
  /** Name of the row being renamed inline, or '' for a new folder row. */
  renaming = $state<string | null>(null);
  #token = 0;

  view = $derived(
    sortEntries(filterEntries(this.entries, this.filter, this.showHidden), this.sort)
  );
  order = $derived(this.view.map((e) => e.name));
  selected = $derived(this.view.filter((e) => this.selection.names.has(e.name)));
  ops = $derived(fsOps(this.endpoint));

  /** Switches endpoint and lands in its home directory. */
  async attach(ep: Endpoint, path?: string | null): Promise<void> {
    this.endpoint = ep;
    this.back = [];
    this.forward = [];
    const home = await fsOps(ep).home();
    if (path && (await this.load(path, false))) return;
    await this.load(home.status === 'ok' ? home.data : '/', false);
  }

  async load(path: string, push = true, showError = true): Promise<boolean> {
    const token = ++this.#token;
    this.loading = true;
    const r = await this.ops.list(path);
    if (token !== this.#token) return false;
    this.loading = false;
    if (r.status === 'error') {
      if (showError) this.error = r.error;
      return false;
    }
    this.error = null;
    if (push && this.path && this.path !== r.data.path) {
      this.back = [...this.back, this.path];
      this.forward = [];
    }
    this.path = r.data.path;
    this.entries = r.data.entries;
    this.selection = prune(this.selection, this.order);
    return true;
  }

  refresh = () => this.load(this.path, false);
  up = () => this.load(parentPath(this.path));

  async goBack(): Promise<void> {
    const prev = this.back.at(-1);
    if (prev === undefined) return;
    const cur = this.path;
    if (await this.load(prev, false)) {
      this.back = this.back.slice(0, -1);
      this.forward = [...this.forward, cur];
    }
  }
  async goForward(): Promise<void> {
    const next = this.forward.at(-1);
    if (next === undefined) return;
    const cur = this.path;
    if (await this.load(next, false)) {
      this.forward = this.forward.slice(0, -1);
      this.back = [...this.back, cur];
    }
  }

  pathOf = (name: string): string => joinPath(this.path, name);

  click(name: string, mods: { shift?: boolean; meta?: boolean }): void {
    this.selection = select(this.selection, this.order, name, mods);
  }
  /** Right-click on an unselected row selects it alone; on a selected row keeps the set. */
  contextOn(name: string): void {
    if (!this.selection.names.has(name)) this.selection = select(emptySelection, this.order, name);
  }
  selectAll(): void {
    this.selection = { names: new Set(this.order), anchor: this.order[0] ?? null };
  }
  clearSelection(): void {
    this.selection = emptySelection;
  }

  toggleSort(key: Sort['key']): void {
    this.sort =
      this.sort.key === key
        ? { key, dir: this.sort.dir === 'asc' ? 'desc' : 'asc' }
        : { key, dir: 'asc' };
  }

  async mkdir(name: string): Promise<ApiError | null> {
    const r = await this.ops.mkdir(this.pathOf(name));
    if (r.status === 'error') return r.error;
    await this.refresh();
    this.selection = select(emptySelection, this.order, name);
    return null;
  }
  async rename(from: string, to: string): Promise<ApiError | null> {
    if (from === to) return null;
    const r = await this.ops.rename(this.pathOf(from), this.pathOf(to));
    if (r.status === 'error') return r.error;
    await this.refresh();
    this.selection = select(emptySelection, this.order, to);
    return null;
  }
  async remove(names: string[]): Promise<ApiError | null> {
    for (const n of names) {
      const r = await this.ops.remove(this.pathOf(n));
      if (r.status === 'error') {
        await this.refresh();
        return r.error;
      }
    }
    this.selection = emptySelection;
    await this.refresh();
    return null;
  }
  async chmod(name: string, mode: number): Promise<ApiError | null> {
    const r = await this.ops.chmod(this.pathOf(name), mode);
    if (!r) return null;
    const res = await r;
    if (res.status === 'error') return res.error;
    await this.refresh();
    return null;
  }
}
