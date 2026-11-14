/** Mock: running port forwards per session (no sockets, just state and events) and ssh_config import. */
import type { Commands } from '../types';
import { db, delay, err, notFound, ok } from './state';
import { sessionLink } from './terminal';
import * as tree from './tree';

/** session id → forward id → bound port */
const running = new Map<string, Map<string, number>>();

export const forwardStart: Commands['forwardStart'] = async (sessionId, forwardId) => {
  await delay();
  const s = sessionLink(sessionId);
  if (!s) return notFound();
  if (!s.hostId) return err({ code: 'Validation', detail: 'forwards need a saved host' });
  const f = db.forwards.find((x) => x.id === forwardId && x.host_id === s.hostId);
  if (!f) return notFound();
  const mine = running.get(sessionId) ?? new Map<string, number>();
  running.set(sessionId, mine);
  const port = f.bind_port || 50000 + mine.size;
  mine.set(forwardId, port);
  s.emit({ type: 'ForwardStatus', forward_id: forwardId, state: 'Active', error: null });
  return ok(port);
};

export const forwardStop: Commands['forwardStop'] = async (sessionId, forwardId) => {
  const s = sessionLink(sessionId);
  if (!s) return notFound();
  if (running.get(sessionId)?.delete(forwardId))
    s.emit({ type: 'ForwardStatus', forward_id: forwardId, state: 'Stopped', error: null });
  return ok(null);
};

export const forwardActive: Commands['forwardActive'] = async (sessionId) => {
  if (!sessionLink(sessionId)) return notFound();
  const mine = running.get(sessionId) ?? new Map<string, number>();
  return ok([...mine].map(([forward_id, port]) => ({ forward_id, port })));
};

/** Mock `~/.ssh/config`: a bastion, a host behind it and one already saved. */
export const sshConfigPreview: Commands['sshConfigPreview'] = async () => {
  await delay();
  const saved = db.hosts[0];
  return ok({
    found: true,
    entries: [
      {
        alias: 'bastion',
        hostname: '203.0.113.10',
        port: null,
        user: 'jump',
        key_path: '~/.ssh/id_ed25519',
        key: 'ok',
        proxy_jump: null,
        forwards: 0,
        unsupported: [],
        duplicate: false
      },
      {
        alias: 'app',
        hostname: 'app.internal',
        port: 2222,
        user: 'deploy',
        key_path: '~/.ssh/id_ed25519',
        key: 'ok',
        proxy_jump: 'bastion',
        forwards: 1,
        unsupported: [],
        duplicate: false
      },
      {
        alias: 'legacy',
        hostname: 'old.example.com',
        port: null,
        user: 'root',
        key_path: '~/.ssh/legacy_rsa',
        key: 'encrypted',
        proxy_jump: null,
        forwards: 0,
        unsupported: ['ProxyCommand'],
        duplicate: false
      },
      ...(saved
        ? [
            {
              alias: saved.label,
              hostname: saved.hostname,
              port: saved.port,
              user: saved.username,
              key_path: null,
              key: 'none' as const,
              proxy_jump: null,
              forwards: 0,
              unsupported: [],
              duplicate: true
            }
          ]
        : [])
    ]
  });
};

export const sshConfigImport: Commands['sshConfigImport'] = async (aliases, folderName) => {
  await delay();
  if (!aliases.length) return err({ code: 'Validation', detail: 'nothing to import' });
  const preview = await sshConfigPreview();
  if (preview.status !== 'ok') return preview;
  const folder = await tree.folderCreate({ name: folderName || 'SSH config' } as never);
  if (folder.status !== 'ok') return folder;
  let hosts = 0;
  for (const e of preview.data.entries.filter((x) => aliases.includes(x.alias))) {
    const r = await tree.hostCreate({
      folder_id: folder.data.id,
      label: e.alias,
      hostname: e.hostname,
      port: e.port,
      username: e.user
    } as never);
    if (r.status === 'ok') hosts++;
  }
  return ok({ folder_id: folder.data.id, hosts, keys: 1, forwards: 1, skipped_keys: 1 });
};
