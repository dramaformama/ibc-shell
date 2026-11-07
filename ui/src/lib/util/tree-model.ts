/** Pure tree model: visible rows, drop targets, optimistic moves, inheritance lookup. */
import type { Folder, FolderDefaults, FolderId, Host, NodeRef, Tree } from '$lib/api';

export type NodeKey = string; // `f:<id>` | `h:<id>`
export const fkey = (id: string): NodeKey => `f:${id}`;
export const hkey = (id: string): NodeKey => `h:${id}`;
export const refKey = (r: NodeRef): NodeKey => (r.kind === 'folder' ? fkey(r.id) : hkey(r.id));
export const keyRef = (k: NodeKey): NodeRef =>
  k.startsWith('f:') ? { kind: 'folder', id: k.slice(2) } : { kind: 'host', id: k.slice(2) };

export interface Row {
  key: NodeKey;
  kind: 'folder' | 'host';
  id: string;
  depth: number;
  parentId: FolderId | null;
  label: string;
  /** Own flag, or the nearest ancestor's (list shows both at full opacity). */
  flag: string | null;
  /** Folder: number of hosts inside (recursive). */
  count: number;
  expanded: boolean;
  hasChildren: boolean;
  folder?: Folder;
  host?: Host;
}

interface Index {
  children: Map<FolderId | null, NodeRef[]>;
  folders: Map<FolderId, Folder>;
  hosts: Map<string, Host>;
}

const cmp = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

export function indexTree(tree: Tree): Index {
  const folders = new Map(tree.folders.map((f) => [f.id, f]));
  const hosts = new Map(tree.hosts.map((h) => [h.id, h]));
  const bucket = new Map<FolderId | null, { ref: NodeRef; key: string; folder: boolean }[]>();
  const push = (p: FolderId | null, e: { ref: NodeRef; key: string; folder: boolean }) =>
    bucket.set(p, [...(bucket.get(p) ?? []), e]);
  for (const f of tree.folders) {
    const p = f.parent_id && folders.has(f.parent_id) ? f.parent_id : null;
    push(p, { ref: { kind: 'folder', id: f.id }, key: f.sort_key, folder: true });
  }
  for (const h of tree.hosts) {
    const p = h.folder_id && folders.has(h.folder_id) ? h.folder_id : null;
    push(p, { ref: { kind: 'host', id: h.id }, key: h.sort_key, folder: false });
  }
  const children = new Map<FolderId | null, NodeRef[]>();
  for (const [p, list] of bucket) {
    list.sort((a, b) => cmp(a.key, b.key) || Number(b.folder) - Number(a.folder));
    children.set(
      p,
      list.map((e) => e.ref)
    );
  }
  return { children, folders, hosts };
}

/** Rows currently visible (collapsed folders hide their subtree). */
export function buildRows(tree: Tree): Row[] {
  const ix = indexTree(tree);
  const counts = new Map<FolderId, number>();
  const countOf = (id: FolderId): number => {
    const known = counts.get(id);
    if (known !== undefined) return known;
    let n = 0;
    for (const c of ix.children.get(id) ?? []) n += c.kind === 'host' ? 1 : countOf(c.id);
    counts.set(id, n);
    return n;
  };
  const rows: Row[] = [];
  const walk = (parent: FolderId | null, depth: number, inherited: string | null) => {
    for (const ref of ix.children.get(parent) ?? []) {
      if (ref.kind === 'host') {
        const host = ix.hosts.get(ref.id);
        if (!host) continue;
        rows.push({
          key: hkey(host.id),
          kind: 'host',
          id: host.id,
          depth,
          parentId: parent,
          label: host.label,
          flag: host.country_code ?? inherited,
          count: 0,
          expanded: false,
          hasChildren: false,
          host
        });
      } else {
        const folder = ix.folders.get(ref.id);
        if (!folder) continue;
        const flag = folder.country_code ?? inherited;
        const kids = ix.children.get(folder.id) ?? [];
        rows.push({
          key: fkey(folder.id),
          kind: 'folder',
          id: folder.id,
          depth,
          parentId: parent,
          label: folder.name,
          flag: folder.country_code,
          count: countOf(folder.id),
          expanded: folder.expanded,
          hasChildren: kids.length > 0,
          folder
        });
        if (folder.expanded) walk(folder.id, depth + 1, flag);
      }
    }
  };
  walk(null, 0, null);
  return rows;
}

/** Ids of `id` and all folders below it. */
export function folderSubtree(tree: Tree, id: FolderId): Set<FolderId> {
  const out = new Set<FolderId>([id]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const f of tree.folders)
      if (f.parent_id && out.has(f.parent_id) && !out.has(f.id)) {
        out.add(f.id);
        grew = true;
      }
  }
  return out;
}

/** Ancestor chain of a folder, nearest first (starting with the folder itself). */
export function folderChain(tree: Tree, id: FolderId | null): Folder[] {
  const byId = new Map(tree.folders.map((f) => [f.id, f]));
  const chain: Folder[] = [];
  for (let cur = id ? byId.get(id) : undefined; cur && chain.length < 16;) {
    chain.push(cur);
    cur = cur.parent_id ? byId.get(cur.parent_id) : undefined;
  }
  return chain;
}

export interface Inherited<T> {
  value: T;
  /** Folder name, or null for a built-in default. */
  from: string | null;
}

/** Nearest ancestor value of a folder-default field, starting at `folderId`. */
export function inheritedDefault<K extends keyof FolderDefaults>(
  tree: Tree,
  folderId: FolderId | null,
  field: K
): Inherited<NonNullable<FolderDefaults[K]>> | null {
  for (const f of folderChain(tree, folderId)) {
    const v = f.defaults[field];
    if (v !== null && v !== undefined) return { value: v, from: f.name };
  }
  return null;
}

/** Nearest ancestor flag starting at `folderId`. */
export function inheritedFlag(tree: Tree, folderId: FolderId | null): Inherited<string> | null {
  for (const f of folderChain(tree, folderId))
    if (f.country_code) return { value: f.country_code, from: f.name };
  return null;
}

// ── Drag and drop ──────────────────────────────────────────

export type DropZone = 'before' | 'inside' | 'after';
export interface Drop {
  parent: FolderId | null;
  after: NodeRef | null;
  /** Row index whose edge/body carries the indicator. */
  rowIndex: number;
  zone: DropZone;
}

/** Siblings of `parent` in order, skipping `skip`. */
export function siblings(tree: Tree, parent: FolderId | null, skip: Set<NodeKey>): NodeRef[] {
  return (indexTree(tree).children.get(parent) ?? []).filter((r) => !skip.has(refKey(r)));
}

/**
 * Translates "pointer over row `index`, in `zone`" into a concrete (parent, after) slot, or null
 * when the drop is invalid (into itself / its own subtree, or inside a host).
 */
export function resolveDrop(
  tree: Tree,
  rows: readonly Row[],
  index: number,
  zone: DropZone,
  dragged: Set<NodeKey>
): Drop | null {
  const row = rows[index];
  if (!row) return null;
  const banned = new Set<FolderId>();
  for (const k of dragged)
    if (k.startsWith('f:')) folderSubtree(tree, k.slice(2)).forEach((x) => banned.add(x));

  if (
    zone === 'inside' ||
    (zone === 'after' && row.kind === 'folder' && row.expanded && row.hasChildren)
  ) {
    if (row.kind !== 'folder' || banned.has(row.id)) return null;
    return {
      parent: row.id,
      after: null,
      rowIndex: index,
      zone: zone === 'inside' ? 'inside' : 'after'
    };
  }
  if (row.parentId && banned.has(row.parentId)) return null;
  const sibs = siblings(tree, row.parentId, dragged);
  if (zone === 'after') return { parent: row.parentId, after: refOf(row), rowIndex: index, zone };
  const pos = sibs.findIndex((r) => refKey(r) === row.key);
  return {
    parent: row.parentId,
    after: pos > 0 ? (sibs[pos - 1] ?? null) : null,
    rowIndex: index,
    zone
  };
}

const refOf = (row: Row): NodeRef => ({ kind: row.kind, id: row.id });

/**
 * Optimistic local move. Sort keys are provisional (`after.sort_key + \u0001`): they sort right
 * after the anchor and are replaced by the server's keys on the follow-up reload.
 */
export function applyMoveLocal(
  tree: Tree,
  moves: { ref: NodeRef; parent: FolderId | null; after: NodeRef | null }[]
): Tree {
  let folders = tree.folders;
  let hosts = tree.hosts;
  for (const m of moves) {
    const anchor = m.after
      ? m.after.kind === 'folder'
        ? folders.find((f) => f.id === m.after!.id)?.sort_key
        : hosts.find((h) => h.id === m.after!.id)?.sort_key
      : undefined;
    const sort_key = anchor === undefined ? '' : anchor + '\u0001';
    if (m.ref.kind === 'folder')
      folders = folders.map((f) =>
        f.id === m.ref.id ? { ...f, parent_id: m.parent, sort_key } : f
      );
    else
      hosts = hosts.map((h) => (h.id === m.ref.id ? { ...h, folder_id: m.parent, sort_key } : h));
  }
  return { folders, hosts };
}
