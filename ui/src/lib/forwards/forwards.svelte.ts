/** Port forward rules per host and which of them run in which session. */
import {
  api,
  type ForwardId,
  type ForwardInput,
  type ForwardPatch,
  type HostId,
  type PortForward,
  type SessionId
} from '$lib/api';
import { errorText } from '$lib/util/errors';
import { ui } from '$lib/stores/ui.svelte';

/** Addresses that only this computer can reach. */
const LOOPBACK = new Set(['127.0.0.1', 'localhost', '::1', '']);
export const isLoopback = (addr: string) => LOOPBACK.has(addr.trim().toLowerCase());

/** `127.0.0.1:8080 → db:5432` style summary of a rule. */
export function route(
  f: Pick<PortForward, 'kind' | 'bind_addr' | 'bind_port' | 'dest_host' | 'dest_port'>,
  port?: number | null
): string {
  const bind = `${isLoopback(f.bind_addr) ? 'localhost' : f.bind_addr}:${port ?? (f.bind_port || 'auto')}`;
  if (f.kind === 'dynamic') return `${bind} · SOCKS5`;
  return `${bind} → ${f.dest_host ?? '?'}:${f.dest_port ?? '?'}`;
}

class ForwardsStore {
  /** host id → rules */
  rules = $state<Record<string, PortForward[]>>({});
  /** session id → forward id → bound port */
  active = $state<Record<string, Record<string, number>>>({});
  /** forward id → last start error */
  errors = $state<Record<string, string>>({});

  async load(host: HostId): Promise<void> {
    const r = await api.forwardList(host);
    if (r.status === 'ok') this.rules[host] = r.data;
  }

  async refresh(sid: SessionId): Promise<void> {
    const r = await api.forwardActive(sid);
    if (r.status === 'ok')
      this.active[sid] = Object.fromEntries(r.data.map((a) => [a.forward_id, a.port]));
  }

  /** `ForwardStatus` from a session's event channel. */
  onStatus(sid: SessionId, id: ForwardId, state: string, error: string | null): void {
    if (state === 'Failed') this.errors[id] = error ?? '';
    else delete this.errors[id];
    void this.refresh(sid);
  }

  /** The session ended: nothing of it runs any more. */
  clear(sid: SessionId): void {
    delete this.active[sid];
  }

  port(sid: SessionId | null, id: ForwardId): number | null {
    return (sid && this.active[sid]?.[id]) || null;
  }

  async start(sid: SessionId, id: ForwardId): Promise<void> {
    const r = await api.forwardStart(sid, id);
    if (r.status === 'error') ui.toast(errorText(r.error), 'error');
    await this.refresh(sid);
  }

  async stop(sid: SessionId, id: ForwardId): Promise<void> {
    await api.forwardStop(sid, id);
    await this.refresh(sid);
  }

  async create(input: ForwardInput): Promise<PortForward | null> {
    const r = await api.forwardCreate(input);
    if (r.status === 'error') {
      ui.toast(errorText(r.error), 'error');
      return null;
    }
    await this.load(input.host_id);
    return r.data;
  }

  async update(host: HostId, id: ForwardId, patch: ForwardPatch): Promise<boolean> {
    const r = await api.forwardUpdate(id, patch);
    if (r.status === 'error') ui.toast(errorText(r.error), 'error');
    await this.load(host);
    return r.status === 'ok';
  }

  async remove(host: HostId, id: ForwardId): Promise<void> {
    const r = await api.forwardDelete(id);
    if (r.status === 'error') ui.toast(errorText(r.error), 'error');
    delete this.errors[id];
    for (const sid of Object.keys(this.active)) delete this.active[sid]?.[id];
    await this.load(host);
  }
}

export const forwards = new ForwardsStore();
