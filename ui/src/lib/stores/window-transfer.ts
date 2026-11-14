import { Channel } from '@tauri-apps/api/core';
import { api, type HostWindowTransfer, type SessionEvent } from '$lib/api';

export const releasedTabs = new Set<string>();
export const fileViews = new Map<
  string,
  () => {
    id: string | null;
    left: string;
    right: string;
    ready: boolean;
  }
>();

type Receiver = { data: (bytes: Uint8Array) => void; event: (event: SessionEvent) => void };
let receiver: Receiver | undefined;
let queued: Array<(target: Receiver) => void> = [];
let pending: HostWindowTransfer | null = null;

export async function receiveWindowTransfer() {
  const data = new Channel<ArrayBuffer>();
  const events = new Channel<SessionEvent>();
  const deliver = (fn: (target: Receiver) => void) => {
    if (receiver) fn(receiver);
    else queued.push(fn);
  };
  data.onmessage = (bytes) => deliver((target) => target.data(new Uint8Array(bytes)));
  events.onmessage = (event) => deliver((target) => target.event(event));
  const result = await api.hostWindowTake(data, events);
  if (result.status === 'ok') pending = result.data;
  return result;
}

export function claimTerminal(target: Receiver) {
  if (!pending?.terminal) return null;
  const { terminal, screen, cwd } = pending;
  pending = { ...pending, terminal: null, screen: null, cwd: null };
  receiver = target;
  const output = queued;
  queued = [];
  return { terminal, screen, cwd, drain: () => output.forEach((fn) => fn(target)) };
}

export function claimFiles() {
  if (!pending?.files) return null;
  const { files, left, right } = pending;
  pending = { ...pending, files: null };
  return { files, left, right };
}
