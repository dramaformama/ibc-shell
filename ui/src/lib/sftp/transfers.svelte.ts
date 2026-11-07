/** Transfer queue state: one subscription to `transfers_subscribe`, folded into a list. */
import { Channel } from '@tauri-apps/api/core';
import {
  api,
  type ConflictId,
  type ConflictInfo,
  type Endpoint,
  type TransferEvent
} from '$lib/api';
import { ema, etaSeconds } from './model';

export type TransferState =
  'queued' | 'running' | 'paused' | 'conflict' | 'done' | 'failed' | 'cancelled';

export interface TransferItem {
  id: string;
  srcPath: string;
  dstPath: string;
  total: number;
  done: number;
  /** EMA-smoothed bytes per second. */
  speed: number;
  state: TransferState;
  skipped: boolean;
  error: string | null;
}

export interface PendingConflict {
  id: string;
  pendingId: ConflictId;
  info: ConflictInfo;
}

/** Bytes counted so far, speed and ETA, derived for display. */
export const etaOf = (t: TransferItem): number | null =>
  t.state === 'running' ? etaSeconds(t.total, t.done, t.speed) : null;

class TransfersStore {
  items = $state.raw<TransferItem[]>([]);
  conflicts = $state.raw<PendingConflict[]>([]);
  open = $state(false);
  /** Bumped whenever a transfer completes, panes refresh their listing on change. */
  doneTick = $state(0);
  #started = false;

  /** Not finished: shown in the status-bar badge. */
  get active(): number {
    return this.items.filter((t) => ['queued', 'running', 'paused', 'conflict'].includes(t.state))
      .length;
  }
  get finished(): number {
    return this.items.filter((t) => ['done', 'failed', 'cancelled'].includes(t.state)).length;
  }

  async start(): Promise<void> {
    if (this.#started) return;
    this.#started = true;
    const ch = new Channel<TransferEvent>();
    ch.onmessage = (e) => this.#apply(e);
    await api.transfersSubscribe(ch);
  }

  #patch(id: string, fn: (t: TransferItem) => Partial<TransferItem>): void {
    this.items = this.items.map((t) => (t.id === id ? { ...t, ...fn(t) } : t));
  }

  #apply(e: TransferEvent): void {
    switch (e.type) {
      case 'Queued':
        this.items = [
          ...this.items,
          {
            id: e.id,
            srcPath: e.src_path,
            dstPath: e.dst_path,
            total: 0,
            done: 0,
            speed: 0,
            state: 'queued',
            skipped: false,
            error: null
          }
        ];
        this.open = true;
        break;
      case 'Started':
        this.#patch(e.id, () => ({ total: e.total_bytes, state: 'running' }));
        break;
      case 'Progress':
        this.#patch(e.id, (t) => ({
          done: e.bytes_done,
          speed: ema(t.speed, e.bytes_per_s),
          state: 'running'
        }));
        break;
      case 'Conflict':
        this.#patch(e.id, () => ({ state: 'conflict' }));
        this.conflicts = [...this.conflicts, { id: e.id, pendingId: e.pending_id, info: e.info }];
        break;
      case 'FileDone':
        if (e.skipped) this.#patch(e.id, () => ({ skipped: true }));
        break;
      case 'Done':
        this.#patch(e.id, (t) => ({ state: 'done', done: t.total, speed: 0 }));
        this.doneTick++;
        break;
      case 'Failed':
        this.#patch(e.id, () => ({ state: 'failed', error: e.message, speed: 0 }));
        break;
      case 'Paused':
        this.#patch(e.id, () => ({ state: 'paused', speed: 0 }));
        break;
      case 'Cancelled':
        this.#patch(e.id, () => ({ state: 'cancelled', speed: 0 }));
        this.conflicts = this.conflicts.filter((c) => c.id !== e.id);
        break;
    }
  }

  async enqueue(src: Endpoint, dst: Endpoint, items: string[], dstDir: string): Promise<boolean> {
    if (!items.length) return true;
    await this.start();
    const r = await api.transferEnqueue({ src, dst, items, dst_dir: dstDir, conflict: 'Ask' });
    return r.status === 'ok';
  }

  pause = (id: string) => api.transferPause(id);
  resume = (id: string) => api.transferResume(id);
  cancel = (id: string) => api.transferCancel(id);
  retry = async (id: string) => {
    this.#patch(id, () => ({ state: 'queued', done: 0, error: null }));
    await api.transferRetry(id);
  };

  async reply(
    c: PendingConflict,
    choice: Parameters<typeof api.transferConflictReply>[1],
    all: boolean
  ) {
    this.conflicts = all ? [] : this.conflicts.filter((x) => x.pendingId !== c.pendingId);
    await api.transferConflictReply(c.pendingId, choice, all);
  }

  clearFinished(): void {
    this.items = this.items.filter((t) => !['done', 'failed', 'cancelled'].includes(t.state));
  }
}

export const transfers = new TransfersStore();
