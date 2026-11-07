/** Mock: folder / host tree commands (`ipc.md` §store / tree). */
import type { Commands, Folder, Host, NodeRef, ResolvedHost, Source } from '../types';
import {
  db,
  delay,
  err,
  folderDepth,
  isSelfOrDescendant,
  keyAfter,
  notFound,
  now,
  ok,
  uuid
} from './state';

const MAX_DEPTH = 8;

/** Applies a patch: `undefined` = untouched; `null` sets null only for nullable keys. */
export function applyPatch<T extends object>(
  target: T,
  patch: object,
  nullable: readonly string[]
): void {
  for (const [k, v] of Object.entries(patch)) {
    if (v === undefined || (typeof v === 'object' && v !== null && !Array.isArray(v))) continue;
    if (v === null && !nullable.includes(k)) continue;
    (target as Record<string, unknown>)[k] = v;
  }
}

function siblingsKey(parent: string | null, after: NodeRef | null): string {
  if (!after) return ' ';
  const node =
    after.kind === 'folder'
      ? db.folders.find((f) => f.id === after.id)
      : db.hosts.find((h) => h.id === after.id);
  const sameParent = node && ('parent_id' in node ? node.parent_id : node.folder_id) === parent;
  return keyAfter(sameParent ? node.sort_key : undefined);
}

function lastKey(parent: string | null): string {
  const keys = [
    ...db.folders.filter((f) => f.parent_id === parent).map((f) => f.sort_key),
    ...db.hosts.filter((h) => h.folder_id === parent).map((h) => h.sort_key)
  ].sort();
  return keyAfter(keys.at(-1));
}

const bySort = <T extends { sort_key: string }>(a: T, b: T) => a.sort_key.localeCompare(b.sort_key);

export const treeList: Commands['treeList'] = async () => {
  await delay();
  return ok({ folders: [...db.folders].sort(bySort), hosts: [...db.hosts].sort(bySort) });
};

export const folderCreate: Commands['folderCreate'] = async (input) => {
  await delay();
  if (!input.name.trim()) return err({ code: 'Validation', detail: 'name is required' });
  if (folderDepth(input.parent_id ?? null) + 1 > MAX_DEPTH) return err({ code: 'TooDeep' });
  const t = now();
  const f: Folder = {
    id: uuid(),
    parent_id: input.parent_id ?? null,
    name: input.name.trim(),
    country_code: input.country_code?.toLowerCase() ?? null,
    color: input.color ?? null,
    icon: input.icon ?? null,
    sort_key: lastKey(input.parent_id ?? null),
    expanded: true,
    defaults: input.defaults ?? {
      username: null,
      port: null,
      identity_id: null,
      jump_host_id: null,
      startup_cmd: null,
      keepalive_s: null,
      term_theme: null
    },
    created_at: t,
    updated_at: t
  };
  db.folders.push(f);
  return ok(structuredClone(f));
};

export const folderUpdate: Commands['folderUpdate'] = async (id, patch) => {
  await delay();
  const f = db.folders.find((x) => x.id === id);
  if (!f) return notFound();
  applyPatch(f, patch, ['country_code', 'color', 'icon']);
  if (patch.defaults) applyPatch(f.defaults, patch.defaults, Object.keys(f.defaults));
  f.updated_at = now();
  return ok(structuredClone(f));
};

export const folderMove: Commands['folderMove'] = async (id, parent, after) => {
  await delay();
  const f = db.folders.find((x) => x.id === id);
  if (!f) return notFound();
  if (isSelfOrDescendant(id, parent)) return err({ code: 'Cycle' });
  if (folderDepth(parent) + 1 > MAX_DEPTH) return err({ code: 'TooDeep' });
  f.parent_id = parent;
  f.sort_key = after ? siblingsKey(parent, after) : ' ';
  return ok(null);
};

export const folderDelete: Commands['folderDelete'] = async (id, mode) => {
  await delay();
  const f = db.folders.find((x) => x.id === id);
  if (!f) return notFound();
  if (mode === 'MoveToParent') {
    for (const c of db.folders) if (c.parent_id === id) c.parent_id = f.parent_id;
    for (const h of db.hosts) if (h.folder_id === id) h.folder_id = f.parent_id;
  } else {
    const doomed = new Set(db.folders.filter((c) => isSelfOrDescendant(id, c.id)).map((c) => c.id));
    db.hosts = db.hosts.filter((h) => !h.folder_id || !doomed.has(h.folder_id));
    db.folders = db.folders.filter((c) => !doomed.has(c.id));
  }
  db.folders = db.folders.filter((c) => c.id !== id);
  return ok(null);
};

export const hostCreate: Commands['hostCreate'] = async (input) => {
  await delay();
  if (!input.label.trim() || !input.hostname.trim())
    return err({ code: 'Validation', detail: 'label and hostname are required' });
  const t = now();
  const h: Host = {
    id: uuid(),
    folder_id: input.folder_id ?? null,
    label: input.label.trim(),
    hostname: input.hostname.trim(),
    port: input.port ?? null,
    username: input.username ?? null,
    identity_id: input.identity_id ?? null,
    jump_host_id: input.jump_host_id ?? null,
    country_code: input.country_code?.toLowerCase() ?? null,
    startup_cmd: input.startup_cmd ?? null,
    keepalive_s: input.keepalive_s ?? null,
    term_theme: input.term_theme ?? null,
    notes: input.notes ?? null,
    tag_ids: input.tag_ids ?? [],
    sort_key: lastKey(input.folder_id ?? null),
    last_connected_at: null,
    created_at: t,
    updated_at: t
  };
  db.hosts.push(h);
  return ok(structuredClone(h));
};

const HOST_NULLABLE = [
  'port',
  'username',
  'identity_id',
  'jump_host_id',
  'country_code',
  'startup_cmd',
  'keepalive_s',
  'term_theme',
  'notes'
] as const;

export const hostUpdate: Commands['hostUpdate'] = async (id, patch) => {
  await delay();
  const h = db.hosts.find((x) => x.id === id);
  if (!h) return notFound();
  applyPatch(h, patch, HOST_NULLABLE);
  if (patch.tag_ids) h.tag_ids = [...patch.tag_ids];
  h.updated_at = now();
  return ok(structuredClone(h));
};

export const hostDuplicate: Commands['hostDuplicate'] = async (id) => {
  await delay();
  const h = db.hosts.find((x) => x.id === id);
  if (!h) return notFound();
  const copy: Host = {
    ...structuredClone(h),
    id: uuid(),
    label: `${h.label} copy`,
    sort_key: keyAfter(h.sort_key)
  };
  db.hosts.push(copy);
  return ok(structuredClone(copy));
};

export const hostDelete: Commands['hostDelete'] = async (ids) => {
  await delay();
  db.hosts = db.hosts.filter((h) => !ids.includes(h.id));
  return ok(null);
};

export const hostMove: Commands['hostMove'] = async (id, folder, after) => {
  await delay();
  const h = db.hosts.find((x) => x.id === id);
  if (!h) return notFound();
  h.folder_id = folder;
  h.sort_key = after ? siblingsKey(folder, after) : ' ';
  return ok(null);
};

/** Walks host → folder chain for a field; mirrors data-model.md §Inheritance resolution. */
function inherit<T>(
  h: Host,
  own: T | null,
  pick: (f: Folder) => T | null
): { value: T | null; source: Source } {
  if (own !== null) return { value: own, source: { kind: 'host' } };
  let cur = h.folder_id;
  while (cur) {
    const f = db.folders.find((x) => x.id === cur);
    if (!f) break;
    const v = pick(f);
    if (v !== null) return { value: v, source: { kind: 'folder', id: f.id } };
    cur = f.parent_id;
  }
  return { value: null, source: { kind: 'default' } };
}

export const hostResolve: Commands['hostResolve'] = async (id) => {
  await delay();
  const h = db.hosts.find((x) => x.id === id);
  if (!h) return notFound();
  const port = inherit(h, h.port, (f) => f.defaults.port);
  const keepalive = inherit(h, h.keepalive_s, (f) => f.defaults.keepalive_s);
  const identity = inherit(h, h.identity_id, (f) => f.defaults.identity_id);
  let username = inherit(h, h.username, (f) => f.defaults.username);
  if (username.value === null && identity.value) {
    const idn = db.identities.find((i) => i.id === identity.value);
    username = { value: idn?.username ?? null, source: { kind: 'default' } };
  }
  const jump = inherit(h, h.jump_host_id, (f) => f.defaults.jump_host_id);
  const resolved: ResolvedHost = {
    id: h.id,
    label: h.label,
    hostname: h.hostname,
    port: { value: port.value ?? 22, source: port.source },
    username,
    identity_id: identity,
    jump_chain: jump.value ? [jump.value] : [],
    country_code: inherit(h, h.country_code, (f) => f.country_code),
    startup_cmd: inherit(h, h.startup_cmd, (f) => f.defaults.startup_cmd),
    keepalive_s: { value: keepalive.value ?? 30, source: keepalive.source },
    term_theme: inherit(h, h.term_theme, (f) => f.defaults.term_theme)
  };
  return ok(resolved);
};

export const hostSearch: Commands['hostSearch'] = async (q, limit) => {
  await delay(5);
  const needle = q.trim().toLowerCase();
  if (!needle) return ok([]);
  const tagName = (id: string) => db.tags.find((t) => t.id === id)?.name ?? '';
  const hits = db.hosts.filter((h) =>
    [h.label, h.hostname, h.notes ?? '', ...h.tag_ids.map(tagName)].some((s) =>
      s.toLowerCase().includes(needle)
    )
  );
  return ok(hits.slice(0, limit).map((h) => h.id));
};
