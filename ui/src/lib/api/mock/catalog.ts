/** Mock: identities, tags, snippets, forwards, known hosts, settings. */
import type { Commands, IdentityDto, PortForward, Snippet, Tag } from '../types';
import { applyPatch } from './tree';
import { db, delay, err, keyAfter, notFound, now, ok, uuid } from './state';
import { vaultIsUnlocked } from './vault';

const locked = () => err<never>({ code: 'VaultLocked' });

export const identityList: Commands['identityList'] = async () => {
  await delay();
  return ok(structuredClone(db.identities));
};

export const identityCreate: Commands['identityCreate'] = async (input) => {
  await delay();
  if (!vaultIsUnlocked()) return locked();
  if (!input.label.trim()) return err({ code: 'Validation', detail: 'label is required' });
  const hasKey = !!input.private_key_pem;
  const t = now();
  const idn: IdentityDto = {
    id: uuid(),
    label: input.label.trim(),
    username: input.username ?? null,
    kind: input.kind,
    key_algo: hasKey ? 'ssh-ed25519' : null,
    key_fingerprint: hasKey ? `SHA256:mock${uuid().replaceAll('-', '').slice(0, 39)}` : null,
    public_key: hasKey ? 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMock imported' : null,
    has_password: !!input.password,
    has_passphrase: !!input.passphrase,
    created_at: t,
    updated_at: t
  };
  db.identities.push(idn);
  return ok(structuredClone(idn));
};

export const identityUpdate: Commands['identityUpdate'] = async (id, patch) => {
  await delay();
  if (!vaultIsUnlocked()) return locked();
  const idn = db.identities.find((i) => i.id === id);
  if (!idn) return notFound();
  if (patch.label) idn.label = patch.label;
  if (patch.username) idn.username = patch.username;
  if (patch.clear_username) idn.username = null;
  if (patch.password) idn.has_password = true;
  if (patch.passphrase) idn.has_passphrase = true;
  if (patch.clear_password) idn.has_password = false;
  if (patch.clear_passphrase) idn.has_passphrase = false;
  idn.updated_at = now();
  return ok(structuredClone(idn));
};

export const identityDelete: Commands['identityDelete'] = async (id) => {
  await delay();
  db.identities = db.identities.filter((i) => i.id !== id);
  for (const h of db.hosts) if (h.identity_id === id) h.identity_id = null;
  return ok(null);
};

export const identityImportFile: Commands['identityImportFile'] = async (passphrase) => {
  return identityCreate({ label: 'id_ed25519', kind: 'key', private_key_pem: 'mock', passphrase });
};

export const tagList: Commands['tagList'] = async () => ok(structuredClone(db.tags));

export const tagCreate: Commands['tagCreate'] = async (input) => {
  await delay();
  if (db.tags.some((t) => t.name.toLowerCase() === input.name.toLowerCase())) {
    return err({ code: 'Validation', detail: 'tag exists' });
  }
  const tag: Tag = { id: uuid(), name: input.name, color: input.color ?? null };
  db.tags.push(tag);
  return ok({ ...tag });
};

export const tagUpdate: Commands['tagUpdate'] = async (id, patch) => {
  const tag = db.tags.find((t) => t.id === id);
  if (!tag) return notFound();
  applyPatch(tag, patch, ['color']);
  return ok({ ...tag });
};

export const tagDelete: Commands['tagDelete'] = async (id) => {
  db.tags = db.tags.filter((t) => t.id !== id);
  db.hosts.forEach((h) => (h.tag_ids = h.tag_ids.filter((t) => t !== id)));
  return ok(null);
};

export const snippetList: Commands['snippetList'] = async (folderId) =>
  ok(structuredClone(db.snippets.filter((s) => s.folder_id === null || s.folder_id === folderId)));

export const snippetCreate: Commands['snippetCreate'] = async (input) => {
  const t = now();
  const s: Snippet = {
    id: uuid(),
    folder_id: input.folder_id ?? null,
    name: input.name,
    body: input.body,
    sort_key: keyAfter(db.snippets.at(-1)?.sort_key),
    created_at: t,
    updated_at: t
  };
  db.snippets.push(s);
  return ok({ ...s });
};

export const snippetUpdate: Commands['snippetUpdate'] = async (id, patch) => {
  const s = db.snippets.find((x) => x.id === id);
  if (!s) return notFound();
  applyPatch(s, patch, ['folder_id']);
  s.updated_at = now();
  return ok({ ...s });
};

export const snippetDelete: Commands['snippetDelete'] = async (id) => {
  db.snippets = db.snippets.filter((s) => s.id !== id);
  return ok(null);
};

export const forwardList: Commands['forwardList'] = async (hostId) =>
  ok(structuredClone(db.forwards.filter((f) => hostId === null || f.host_id === hostId)));

export const forwardCreate: Commands['forwardCreate'] = async (input) => {
  const f: PortForward = {
    id: uuid(),
    host_id: input.host_id,
    kind: input.kind,
    bind_addr: input.bind_addr ?? '127.0.0.1',
    bind_port: input.bind_port,
    dest_host: input.dest_host ?? null,
    dest_port: input.dest_port ?? null,
    auto_start: input.auto_start ?? false,
    label: input.label ?? null
  };
  db.forwards.push(f);
  return ok({ ...f });
};

export const forwardUpdate: Commands['forwardUpdate'] = async (id, patch) => {
  const f = db.forwards.find((x) => x.id === id);
  if (!f) return notFound();
  applyPatch(f, patch, ['dest_host', 'dest_port', 'label']);
  return ok({ ...f });
};

export const forwardDelete: Commands['forwardDelete'] = async (id) => {
  db.forwards = db.forwards.filter((f) => f.id !== id);
  return ok(null);
};

export const knownHostList: Commands['knownHostList'] = async () =>
  ok(structuredClone(db.knownHosts));

export const knownHostDelete: Commands['knownHostDelete'] = async (id) => {
  db.knownHosts = db.knownHosts.filter((k) => k.id !== id);
  return ok(null);
};

export const settingsGet: Commands['settingsGet'] = async (key) =>
  ok(structuredClone(db.settings.get(key) ?? null));

export const settingsSet: Commands['settingsSet'] = async (key, value) => {
  db.settings.set(key, value);
  return ok(null);
};
