/** Automatic reconnect after a dropped connection: backoff, countdown, network-back trigger. */

/** Pause before each attempt, seconds. After the last one the user reconnects by hand. */
export const BACKOFF = [1, 2, 4, 8, 15, 30, 30, 30, 30, 30] as const;

/** Disconnect reasons a new attempt can fix (not `exit`, not a user close, not a protocol error). */
export function retriableReason(reason: string | null): boolean {
  if (!reason) return false;
  return (
    reason.startsWith('network:') ||
    reason.startsWith('server:') ||
    reason === 'keepalive' ||
    reason === 'connection closed'
  );
}

/** Failure codes of an attempt worth another one (the server may still be booting). */
export const retriableFailure = (code: string) => code === 'Network' || code === 'Timeout';

export class AutoReconnect {
  /** Attempts made since the drop (0 = idle). */
  attempt = $state(0);
  /** When the next attempt starts (ms), `null` when none is scheduled. */
  dueAt = $state<number | null>(null);
  /** Ticks once a second while counting down. */
  clock = $state(Date.now());
  #timer: ReturnType<typeof setTimeout> | undefined;
  #tick: ReturnType<typeof setInterval> | undefined;
  #run: () => void;
  #onBack = () => this.now();

  constructor(run: () => void) {
    this.#run = run;
  }

  get waiting(): boolean {
    return this.dueAt !== null;
  }

  get seconds(): number {
    return this.dueAt === null ? 0 : Math.max(0, Math.ceil((this.dueAt - this.clock) / 1000));
  }

  /** Plans the next attempt; `false` when every attempt is used up. */
  schedule(): boolean {
    this.#clear();
    const pause = BACKOFF[this.attempt];
    if (pause === undefined) {
      this.attempt = 0;
      return false;
    }
    this.clock = Date.now();
    this.dueAt = this.clock + pause * 1000;
    this.#timer = setTimeout(() => this.now(), pause * 1000);
    this.#tick = setInterval(() => (this.clock = Date.now()), 1000);
    if (typeof window !== 'undefined') {
      window.addEventListener('online', this.#onBack);
      document.addEventListener('visibilitychange', this.#onVisible);
    }
    return true;
  }

  /** Starts the planned attempt right away. */
  now(): void {
    if (this.dueAt === null) return;
    this.#clear();
    this.attempt++;
    this.#run();
  }

  /** Connected again, or the user gave up / closed the tab. */
  cancel(): void {
    this.#clear();
    this.attempt = 0;
  }

  #onVisible = () => {
    if (document.visibilityState === 'visible' && navigator.onLine) this.now();
  };

  #clear(): void {
    clearTimeout(this.#timer);
    clearInterval(this.#tick);
    this.dueAt = null;
    if (typeof window !== 'undefined') {
      window.removeEventListener('online', this.#onBack);
      document.removeEventListener('visibilitychange', this.#onVisible);
    }
  }
}
