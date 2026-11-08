/**
 * What happened during one connect, step by step: which step ran, how long it took, which
 * addresses were tried, where it stopped. Fed with the session's events; no DOM.
 */
import type { ApiError, ConnectStep, SessionEvent } from '$lib/api';
import type { Attempt } from './diagnose';

export type RowState = 'active' | 'waiting' | 'ok' | 'failed';

export interface Row {
  id: number;
  step: ConnectStep;
  /** Label of the host the step belongs to. */
  hop: string;
  state: RowState;
  attempts: Attempt[];
  /** Milliseconds spent (not counting time spent waiting for the user), once finished. */
  ms: number | null;
}

interface Clock {
  start: number;
  /** Time spent on prompts so far. */
  waited: number;
  /** When the current prompt appeared. */
  waitFrom: number | null;
}

export class ConnectLog {
  rows = $state<Row[]>([]);
  /** The error that ended the connect, once it failed. */
  error = $state.raw<ApiError | null>(null);
  /** Ticks while a step runs, so the UI can show "waiting for 7 s". */
  now = $state(performance.now());
  /** Total connect time in ms (without prompts), once connected. */
  total = $state<number | null>(null);

  #clock = new Map<number, Clock>();
  #seq = 0;
  #timer: ReturnType<typeof setInterval> | undefined;

  /** Starts over (a new connect). */
  reset(): void {
    this.#stop();
    this.rows = [];
    this.error = null;
    this.total = null;
    this.#clock.clear();
  }

  get hops(): string[] {
    return [...new Set(this.rows.map((r) => r.hop))];
  }

  get last(): Row | null {
    return this.rows.at(-1) ?? null;
  }

  /** The failing step is on a later hop than the first one. */
  get viaJump(): boolean {
    const l = this.last;
    return !!l && this.hops.length > 1 && l.hop !== this.hops[0];
  }

  /** Milliseconds a row has been running (or took), without prompt time. */
  elapsed(row: Row): number {
    if (row.ms !== null) return row.ms;
    const c = this.#clock.get(row.id);
    if (!c) return 0;
    const upto = c.waitFrom ?? this.now;
    return Math.max(0, upto - c.start - c.waited);
  }

  feed(e: SessionEvent): void {
    switch (e.type) {
      case 'Connecting':
        this.#end('ok');
        this.#begin(e.step, e.hop);
        break;
      case 'Attempt':
        this.last?.attempts.push({ addr: e.addr, failure: e.failure });
        break;
      case 'HostKeyPrompt':
      case 'KbdInteractive':
      case 'PassphrasePrompt':
        this.#wait();
        break;
      case 'Connected':
        this.#end('ok');
        this.total = this.rows.reduce((s, r) => s + (r.ms ?? 0), 0);
        this.#stop();
        break;
      case 'Failed':
        this.#end('failed');
        this.error = e.error;
        this.#stop();
        break;
      default:
        break;
    }
  }

  /** A prompt was answered: the step runs again. */
  resume(): void {
    const l = this.last;
    const c = l && this.#clock.get(l.id);
    if (!l || !c || c.waitFrom === null) return;
    c.waited += performance.now() - c.waitFrom;
    c.waitFrom = null;
    l.state = 'active';
  }

  /** Stops the ticker (the tab is closed). */
  dispose(): void {
    this.#stop();
  }

  /** Plain-text report for support: steps, times, addresses and the failure. No secrets. */
  report(target: string, summary: string | null): string {
    const out = [`iBC Shell · ${target}`];
    for (const r of this.rows) {
      const ms = r.ms === null ? '…' : `${Math.round(r.ms)} ms`;
      const mark = r.state === 'ok' ? 'ok' : r.state === 'failed' ? 'FAILED' : r.state;
      out.push(`${r.hop} · ${r.step} · ${mark} · ${ms}`);
      for (const a of r.attempts) out.push(`  ${a.addr} → ${a.failure ?? 'connected'}`);
    }
    if (this.error) out.push(`error: ${JSON.stringify(this.error)}`);
    if (summary) out.push(summary);
    return out.join('\n');
  }

  #begin(step: ConnectStep, hop: string): void {
    const id = ++this.#seq;
    this.rows.push({ id, step, hop, state: 'active', attempts: [], ms: null });
    this.#clock.set(id, { start: performance.now(), waited: 0, waitFrom: null });
    this.now = performance.now();
    this.#timer ??= setInterval(() => (this.now = performance.now()), 250);
  }

  #wait(): void {
    const l = this.last;
    const c = l && this.#clock.get(l.id);
    if (!l || !c || c.waitFrom !== null) return;
    c.waitFrom = performance.now();
    l.state = 'waiting';
  }

  #end(state: 'ok' | 'failed'): void {
    this.resume();
    const l = this.last;
    if (!l || (l.state !== 'active' && l.state !== 'waiting')) return;
    this.now = performance.now();
    l.ms = this.elapsed({ ...l, ms: null });
    l.state = state;
  }

  #stop(): void {
    clearInterval(this.#timer);
    this.#timer = undefined;
  }
}
