import { Channel } from '@tauri-apps/api/core';
import { api, type HostWindowTransfer, type Result, type SessionEvent } from '$lib/api';
import { groupKey, type Tab } from './sessions.svelte';

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

/** Sessions handed over by another window, waiting for the tabs of one group to claim them. */
interface Inbox {
  payload: HostWindowTransfer | null;
  receiver?: Receiver;
  queued: Array<(target: Receiver) => void>;
}

/** Keyed by tab group (`groupKey`), so a returned host never lands in another host's tab. */
const inboxes = new Map<string, Inbox>();

async function receive(
  key: string,
  take: (
    data: Channel<ArrayBuffer>,
    events: Channel<SessionEvent>
  ) => Promise<Result<HostWindowTransfer | null>>
) {
  const inbox: Inbox = { payload: null, queued: [] };
  inboxes.set(key, inbox);
  const data = new Channel<ArrayBuffer>();
  const events = new Channel<SessionEvent>();
  const deliver = (fn: (target: Receiver) => void) => {
    if (inbox.receiver) fn(inbox.receiver);
    else inbox.queued.push(fn);
  };
  data.onmessage = (bytes) => deliver((target) => target.data(new Uint8Array(bytes)));
  events.onmessage = (event) => deliver((target) => target.event(event));
  const result = await take(data, events);
  if (result.status === 'ok' && result.data) inbox.payload = result.data;
  else if (inboxes.get(key) === inbox) inboxes.delete(key);
  return result;
}

/** Host window start-up: takes what the main window handed over (first instance of the host). */
export function receiveWindowTransfer(hostId: string) {
  return receive(`h:${hostId}`, (data, events) => api.hostWindowTake(data, events));
}

/** Main window: takes the sessions a host window returned, for the group `key`. */
export function adoptWindowReturn(label: string, key: string) {
  return receive(key, (data, events) => api.hostWindowAdopt(label, data, events));
}

function inboxFor(tab: Tab) {
  const key = groupKey(tab);
  const inbox = inboxes.get(key);
  return inbox?.payload ? { key, inbox, payload: inbox.payload } : null;
}

/** Everything claimed: later tabs of the group connect on their own (output keeps flowing). */
function settle(key: string, inbox: Inbox) {
  if (!inbox.payload?.terminal && !inbox.payload?.files) inboxes.delete(key);
}

export function claimTerminal(tab: Tab, target: Receiver) {
  const found = inboxFor(tab);
  if (!found?.payload.terminal) return null;
  const { key, inbox, payload } = found;
  const { terminal, screen, cwd } = payload;
  inbox.payload = { ...payload, terminal: null, screen: null, cwd: null };
  inbox.receiver = target;
  const output = inbox.queued;
  inbox.queued = [];
  settle(key, inbox);
  return { terminal, screen, cwd, drain: () => output.forEach((fn) => fn(target)) };
}

export function claimFiles(tab: Tab) {
  const found = inboxFor(tab);
  if (!found?.payload.files) return null;
  const { key, inbox, payload } = found;
  const { files, left, right } = payload;
  inbox.payload = { ...payload, files: null };
  settle(key, inbox);
  return { files, left, right };
}
