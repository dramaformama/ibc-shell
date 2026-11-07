/** Mock: SFTP, local FS and the transfer queue (simulated progress ≈ 25 MB/s, 4 Hz). */
import type { Channel } from '@tauri-apps/api/core';
import type { Commands, ConflictChoice, Endpoint, TransferEvent } from '../types';
import { basename, join, localSeed, MemFs, normalize, remoteSeed } from './fs';
import { db, delay, err, notFound, ok, uuid } from './state';
import { hasSession } from './terminal';

const remoteFs = remoteSeed(); // shared by every mock SFTP session
const localFs = localSeed();
const sftps = new Set<string>();

const fsOf = (sftpId: string): MemFs | null => (sftps.has(sftpId) ? remoteFs : null);
const noSftp = () => err<never>({ code: 'Sftp', detail: 'unknown sftp session' });
const exists = () => err<never>({ code: 'Validation', detail: 'already exists' });

export const sftpOpen: Commands['sftpOpen'] = async (target, onEvent) => {
  await delay(150);
  if (target.kind === 'session' && !hasSession(target.session_id)) return notFound();
  if (target.kind === 'host' && !db.hosts.some((h) => h.id === target.host_id)) return notFound();
  const id = uuid();
  sftps.add(id);
  onEvent.onmessage({ type: 'Connected', server_banner: null });
  return ok(id);
};

export const sftpClose: Commands['sftpClose'] = async (sftpId) =>
  sftps.delete(sftpId) ? ok(null) : noSftp();

export const sftpList: Commands['sftpList'] = async (sftpId, path) => {
  await delay(60);
  const fs = fsOf(sftpId);
  if (!fs) return noSftp();
  return fs.list(path) ? ok(fs.list(path)!) : notFound();
};

export const sftpStat: Commands['sftpStat'] = async (sftpId, path) => {
  const e = fsOf(sftpId)?.entry(path);
  return e ? ok(e) : notFound();
};

export const sftpMkdir: Commands['sftpMkdir'] = async (sftpId, path) => {
  const fs = fsOf(sftpId);
  if (!fs) return noSftp();
  if (fs.get(path)) return exists();
  fs.add(path, { kind: 'dir', size: 4096, mtime: Date.now(), mode: 0o40755, link_target: null });
  return ok(null);
};

export const sftpRename: Commands['sftpRename'] = async (sftpId, from, to) => {
  const fs = fsOf(sftpId);
  if (!fs) return noSftp();
  if (!fs.get(from)) return notFound();
  if (fs.get(to)) return exists();
  fs.rename(from, to);
  return ok(null);
};

export const sftpRemove: Commands['sftpRemove'] = async (sftpId, path, recursive) => {
  const fs = fsOf(sftpId);
  if (!fs) return noSftp();
  const n = fs.get(path);
  if (!n) return notFound();
  if (n.kind === 'dir' && !recursive && (fs.list(path)?.entries.length ?? 0) > 0) {
    return err({ code: 'Sftp', detail: 'directory not empty' });
  }
  fs.remove(path);
  return ok(null);
};

export const sftpChmod: Commands['sftpChmod'] = async (sftpId, path, mode) => {
  const n = fsOf(sftpId)?.get(path);
  if (!n) return notFound();
  n.mode = (n.mode & ~0o7777) | (mode & 0o7777);
  return ok(null);
};

export const sftpRealpath: Commands['sftpRealpath'] = async (sftpId, path) => {
  const fs = fsOf(sftpId);
  if (!fs) return noSftp();
  return ok(path === '.' || path === '' ? fs.home : normalize(path));
};

export const localHome: Commands['localHome'] = async () => ok(localFs.home);

export const localList: Commands['localList'] = async (path) => {
  await delay(10);
  const l = localFs.list(path);
  return l ? ok(l) : notFound();
};

export const localMkdir: Commands['localMkdir'] = async (path) => {
  if (localFs.get(path)) return exists();
  localFs.add(path, {
    kind: 'dir',
    size: 4096,
    mtime: Date.now(),
    mode: 0o40755,
    link_target: null
  });
  return ok(null);
};

export const localRename: Commands['localRename'] = async (from, to) => {
  if (!localFs.get(from)) return notFound();
  localFs.rename(from, to);
  return ok(null);
};

export const localRemove: Commands['localRemove'] = async (path) => {
  if (!localFs.get(path)) return notFound();
  localFs.remove(path);
  return ok(null);
};

// ---- transfers ----
interface Transfer {
  id: string;
  src: MemFs;
  dst: MemFs;
  srcPath: string;
  dstPath: string;
  total: number;
  done: number;
  state: 'queued' | 'running' | 'paused' | 'conflict' | 'done' | 'failed' | 'cancelled';
  choice: ConflictChoice | null;
}

const transfers = new Map<string, Transfer>();
const conflicts = new Map<string, string>(); // pending_id → transfer id
let subscriber: Channel<TransferEvent> | null = null;
const send = (e: TransferEvent) => subscriber?.onmessage(e);
const BYTES_PER_TICK = 6_250_000; // 25 MB/s at 4 Hz

const endpointFs = (e: Endpoint): MemFs | null => (e.kind === 'local' ? localFs : fsOf(e.sftp_id));

function tick(t: Transfer): void {
  if (t.state !== 'running') return;
  t.done = Math.min(t.total, t.done + BYTES_PER_TICK);
  send({ type: 'Progress', id: t.id, bytes_done: t.done, bytes_per_s: BYTES_PER_TICK * 4 });
  if (t.done < t.total) {
    setTimeout(() => tick(t), 250);
    return;
  }
  for (const k of t.src.subtree(t.srcPath)) {
    t.dst.add(t.dstPath + k.slice(normalize(t.srcPath).length), { ...t.src.nodes.get(k)! });
  }
  send({ type: 'FileDone', id: t.id, path: t.dstPath, skipped: false });
  t.state = 'done';
  send({ type: 'Done', id: t.id });
}

function start(t: Transfer): void {
  if (t.dst.get(t.dstPath) && t.choice === null) {
    const pending_id = uuid();
    conflicts.set(pending_id, t.id);
    t.state = 'conflict';
    const s = t.src.entry(t.srcPath)!;
    const d = t.dst.entry(t.dstPath)!;
    send({
      type: 'Conflict',
      id: t.id,
      pending_id,
      info: {
        path: t.dstPath,
        src_size: s.size,
        src_mtime: s.mtime,
        dst_size: d.size,
        dst_mtime: d.mtime
      }
    });
    return;
  }
  if (t.choice === 'Skip') {
    t.state = 'done';
    send({ type: 'FileDone', id: t.id, path: t.dstPath, skipped: true });
    send({ type: 'Done', id: t.id });
    return;
  }
  if (t.choice === 'Rename') t.dstPath = `${t.dstPath} (1)`;
  if (t.choice === 'Resume') t.done = Math.min(t.total, t.dst.get(t.dstPath)?.size ?? 0);
  t.state = 'running';
  const m = t.src.measure(t.srcPath);
  send({ type: 'Started', id: t.id, total_bytes: m.bytes, files: m.files });
  setTimeout(() => tick(t), 250);
}

export const transferEnqueue: Commands['transferEnqueue'] = async (req) => {
  const src = endpointFs(req.src);
  const dst = endpointFs(req.dst);
  if (!src || !dst) return noSftp();
  if (!req.items.every((p) => src.get(p))) return notFound();
  const policyChoice: Record<string, ConflictChoice | null> = {
    Ask: null,
    Overwrite: 'Overwrite',
    Skip: 'Skip',
    Rename: 'Rename',
    ResumeIfSmaller: 'Resume'
  };
  const ids = req.items.map((srcPath) => {
    const t: Transfer = {
      id: uuid(),
      src,
      dst,
      srcPath: normalize(srcPath),
      dstPath: join(req.dst_dir, basename(srcPath)),
      total: src.measure(srcPath).bytes,
      done: 0,
      state: 'queued',
      choice: policyChoice[req.conflict] ?? null
    };
    transfers.set(t.id, t);
    send({ type: 'Queued', id: t.id, src_path: t.srcPath, dst_path: t.dstPath });
    setTimeout(() => start(t), 50);
    return t.id;
  });
  return ok(ids);
};

function control(id: string, fn: (t: Transfer) => void) {
  const t = transfers.get(id);
  if (!t) return notFound();
  fn(t);
  return ok(null);
}

export const transferPause: Commands['transferPause'] = async (id) =>
  control(id, (t) => {
    if (t.state !== 'running') return;
    t.state = 'paused';
    send({ type: 'Paused', id });
  });

export const transferResume: Commands['transferResume'] = async (id) =>
  control(id, (t) => {
    if (t.state !== 'paused') return;
    t.state = 'running';
    tick(t);
  });

export const transferCancel: Commands['transferCancel'] = async (id) =>
  control(id, (t) => {
    if (t.state === 'done') return;
    t.state = 'cancelled';
    send({ type: 'Cancelled', id });
  });

export const transferRetry: Commands['transferRetry'] = async (id) =>
  control(id, (t) => {
    if (t.state !== 'failed' && t.state !== 'cancelled') return;
    t.done = 0;
    start(t);
  });

export const transferConflictReply: Commands['transferConflictReply'] = async (
  pendingId,
  choice,
  applyToAll
) => {
  const id = conflicts.get(pendingId);
  const t = id && transfers.get(id);
  if (!t) return notFound();
  conflicts.delete(pendingId);
  t.choice = choice;
  if (applyToAll) {
    for (const [pid, tid] of conflicts) {
      const other = transfers.get(tid);
      if (!other) continue;
      other.choice = choice;
      conflicts.delete(pid);
      start(other);
    }
  }
  start(t);
  return ok(null);
};

export const transfersSubscribe: Commands['transfersSubscribe'] = async (onEvent) => {
  subscriber = onEvent;
  return ok(null);
};
