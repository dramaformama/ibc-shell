/**
 * One SSH session behind a terminal tab: IPC channels, connect progress, prompts,
 * credit-window acks, rAF-batched input and debounced resize. No DOM, no xterm.
 */
import { Channel } from '@tauri-apps/api/core';
import {
  api,
  type ApiError,
  type ConnectStep,
  type HostKeyInfo,
  type HostKeyMismatch,
  type KbdPrompt,
  type SessionEvent,
  type SessionId
} from '$lib/api';
import type { Tab } from '$lib/stores/sessions.svelte';
import { ConnectLog } from './connectlog.svelte';
import { concat } from './text';

export type Phase = 'connecting' | 'connected' | 'disconnected' | 'failed';

export type Prompt =
  | { kind: 'hostkey'; pendingId: string; info: HostKeyInfo }
  | { kind: 'changed'; mismatch: HostKeyMismatch }
  | { kind: 'kbd'; pendingId: string; name: string; instruction: string; prompts: KbdPrompt[] }
  | { kind: 'passphrase'; pendingId: string; label: string };

/** `Channel` needs the Tauri runtime; the browser mock only needs an object with `onmessage`. */
function makeChannel<T>(onmessage: (m: T) => void): Channel<T> {
  try {
    const c = new Channel<T>();
    c.onmessage = onmessage;
    return c;
  } catch {
    return { onmessage } as unknown as Channel<T>;
  }
}

const ACK_FLUSH = 64 * 1024;

export class Connection {
  phase = $state<Phase>('connecting');
  step = $state<ConnectStep | null>(null);
  prompt = $state.raw<Prompt | null>(null);
  /** Reason shown in the banner (disconnect reason or localized-free failure code). */
  reason = $state<string | null>(null);
  canReconnect = $state(false);
  /** Steps of the current connect, with times and addresses. */
  readonly log = new ConnectLog();

  #id: SessionId | null = null;
  #tab: Tab;
  #sink: (data: Uint8Array, done: () => void) => void;
  #cols = 80;
  #rows = 24;
  #sentSize = '';
  #resizeTimer: ReturnType<typeof setTimeout> | undefined;
  #input: Uint8Array[] = [];
  #raf = 0;
  #ackDue = 0;
  #ackQueued = false;
  #closed = false;
  #generation = 0;

  constructor(tab: Tab, sink: (data: Uint8Array, done: () => void) => void) {
    this.#tab = tab;
    this.#sink = sink;
  }

  get id(): SessionId | null {
    return this.#id;
  }

  /** Open the session at the given grid size. */
  async open(cols: number, rows: number): Promise<void> {
    this.#cols = cols;
    this.#rows = rows;
    this.#sentSize = `${cols}x${rows}`;
    this.phase = 'connecting';
    this.step = null;
    this.reason = null;
    this.prompt = null;
    this.log.reset();
    const gen = ++this.#generation;
    const onData = makeChannel<ArrayBuffer>((buf) => {
      if (gen === this.#generation) this.#receive(new Uint8Array(buf));
    });
    const onEvent = makeChannel<SessionEvent>((e) => {
      if (gen === this.#generation) this.#event(e);
    });
    const t = this.#tab;
    const r = t.hostId
      ? await api.sessionOpen(t.hostId, cols, rows, onData, onEvent)
      : t.adhoc
        ? await api.sessionOpenAdhoc(t.adhoc, cols, rows, onData, onEvent)
        : null;
    if (this.#closed) {
      if (r?.status === 'ok') void api.sessionClose(r.data);
      return;
    }
    if (r?.status === 'ok') this.#id = r.data;
    else {
      const error: ApiError = r?.status === 'error' ? r.error : { code: 'Internal', detail: '' };
      this.log.feed({ type: 'Failed', error });
      this.#fail(error.code);
    }
  }

  #receive(data: Uint8Array): void {
    this.#sink(data, () => {
      this.#ackDue += data.length;
      if (this.#ackDue >= ACK_FLUSH) this.#flushAck();
      else if (!this.#ackQueued) {
        this.#ackQueued = true;
        queueMicrotask(() => this.#flushAck());
      }
    });
  }

  #flushAck(): void {
    this.#ackQueued = false;
    const n = this.#ackDue;
    if (!n || !this.#id) return;
    this.#ackDue = 0;
    void api.shellAck(this.#id, n);
  }

  #fail(code: string): void {
    this.phase = 'failed';
    this.prompt = null;
    this.reason = code;
    this.canReconnect = false;
  }

  #event(e: SessionEvent): void {
    this.log.feed(e);
    switch (e.type) {
      case 'Connecting':
        this.step = e.step;
        break;
      case 'HostKeyPrompt':
        this.prompt = { kind: 'hostkey', pendingId: e.pending_id, info: e.info };
        break;
      case 'KbdInteractive':
        this.prompt = {
          kind: 'kbd',
          pendingId: e.pending_id,
          name: e.name,
          instruction: e.instruction,
          prompts: e.prompts
        };
        break;
      case 'PassphrasePrompt':
        this.prompt = { kind: 'passphrase', pendingId: e.pending_id, label: e.identity_label };
        break;
      case 'Connected':
        this.phase = 'connected';
        this.prompt = null;
        this.sendSize(true);
        break;
      case 'Failed':
        if (e.error.code === 'HostKeyChanged') {
          this.phase = 'failed';
          this.reason = e.error.code;
          this.prompt = { kind: 'changed', mismatch: e.error.detail };
        } else this.#fail(e.error.code);
        break;
      case 'Disconnected':
        this.phase = 'disconnected';
        this.prompt = null;
        this.reason = e.reason;
        this.canReconnect = e.can_reconnect;
        break;
      default:
        break;
    }
  }

  // ── prompt replies (answers are passed through once and never stored) ──

  async acceptHostKey(remember: boolean): Promise<void> {
    const p = this.prompt;
    if (p?.kind !== 'hostkey') return;
    this.prompt = null;
    this.log.resume();
    await api.hostkeyAccept(p.pendingId, remember);
  }

  /** Reject = close the session (contract: reject is `session_close`). */
  async rejectHostKey(): Promise<void> {
    this.prompt = null;
    this.log.feed({ type: 'Failed', error: { code: 'Cancelled' } });
    this.#fail('Cancelled');
    if (this.#id) await api.sessionClose(this.#id);
    this.#id = null;
  }

  async answerKbd(answers: string[] | null): Promise<void> {
    const p = this.prompt;
    if (p?.kind !== 'kbd') return;
    this.prompt = null;
    this.log.resume();
    await api.kbdInteractiveReply(p.pendingId, answers);
  }

  async answerPassphrase(passphrase: string | null): Promise<void> {
    const p = this.prompt;
    if (p?.kind !== 'passphrase') return;
    this.prompt = null;
    this.log.resume();
    await api.passphraseReply(p.pendingId, passphrase);
  }

  // ── data path ──

  /** Queue keystrokes; one `shell_write` per animation frame. */
  write(bytes: Uint8Array): void {
    if (this.phase !== 'connected' || !this.#id) return;
    this.#input.push(bytes);
    if (this.#raf) return;
    this.#raf = requestAnimationFrame(() => {
      this.#raf = 0;
      const chunks = this.#input;
      this.#input = [];
      if (this.#id && chunks.length) void api.shellWrite(this.#id, concat(chunks));
    });
  }

  /** Grid size changed; debounced 50 ms, only sent when cols/rows differ from the last sent. */
  resize(cols: number, rows: number): void {
    this.#cols = cols;
    this.#rows = rows;
    clearTimeout(this.#resizeTimer);
    this.#resizeTimer = setTimeout(() => this.sendSize(false), 50);
  }

  sendSize(force: boolean): void {
    const key = `${this.#cols}x${this.#rows}`;
    if (!this.#id || (!force && key === this.#sentSize)) return;
    this.#sentSize = key;
    void api.shellResize(this.#id, this.#cols, this.#rows);
  }

  /** Reuse the connection after `Disconnected`. */
  async reconnect(): Promise<void> {
    if (!this.#id || this.phase !== 'disconnected' || !this.canReconnect) return;
    this.phase = 'connecting';
    this.step = null;
    this.reason = null;
    this.log.reset();
    await api.sessionReconnect(this.#id);
  }

  /** Start over with a fresh session (after a failure). */
  async retry(): Promise<void> {
    const old = this.#id;
    this.#id = null;
    if (old) await api.sessionClose(old);
    await this.open(this.#cols, this.#rows);
  }

  close(): void {
    this.#closed = true;
    this.#generation++;
    this.log.dispose();
    clearTimeout(this.#resizeTimer);
    if (this.#raf) cancelAnimationFrame(this.#raf);
    if (this.#id) void api.sessionClose(this.#id);
    this.#id = null;
  }
}
