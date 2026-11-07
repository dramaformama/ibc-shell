/** Visible-host reachability: shared observers, bounded concurrency and cached observations. */
import { api, type HostReachability } from '$lib/api';
import { hidden, whenVisible } from '$lib/util/visible';

/** Every 3 min: each probe is a pre-auth disconnect in the server's sshd log (ADR-010 amendment). */
const EVERY = 180_000;

export class ReachabilityStore {
  data = $state<Record<string, HostReachability | null>>({});
  #watchers = new Map<string, number>();
  #running = new Map<string, object>();
  #active = 0;
  #epoch = 0;
  #queue: (() => void)[] = [];

  /** One observation loop per visible host; no login or SSH trust changes. */
  watch(id: string): () => void {
    this.#watchers.set(id, (this.#watchers.get(id) ?? 0) + 1);
    if (!this.#running.has(id)) {
      const token = {};
      this.#running.set(id, token);
      void this.#loop(id, token).finally(() => {
        if (this.#running.get(id) === token) this.#running.delete(id);
      });
    }
    const epoch = this.#epoch;
    let stopped = false;
    return () => {
      if (stopped || epoch !== this.#epoch) return;
      stopped = true;
      const count = (this.#watchers.get(id) ?? 1) - 1;
      if (count > 0) this.#watchers.set(id, count);
      else this.#watchers.delete(id);
    };
  }

  async #acquire(): Promise<() => void> {
    if (this.#active < 4) this.#active++;
    else await new Promise<void>((resolve) => this.#queue.push(resolve));
    return () => {
      const next = this.#queue.shift();
      if (next) next();
      else this.#active--;
    };
  }

  async #loop(id: string, token: object): Promise<void> {
    const live = () => this.#watchers.has(id) && this.#running.get(id) === token;
    while (live()) {
      if (hidden()) {
        await whenVisible();
        if (!live()) break;
      }
      const prior = this.data[id];
      const age = prior ? Date.now() - prior.checked_at : EVERY;
      if (age < EVERY) {
        await new Promise((resolve) => setTimeout(resolve, EVERY - age));
        continue;
      }
      const release = await this.#acquire();
      if (!live()) {
        release();
        break;
      }
      let stop: boolean;
      try {
        const r = await api.hostProbe(id);
        if (live()) this.data[id] = r.status === 'ok' ? r.data : null;
        stop = r.status === 'error';
      } catch {
        if (live()) this.data[id] = null;
        stop = true;
      } finally {
        release();
      }
      if (stop || !live()) break;
      await new Promise((resolve) => setTimeout(resolve, EVERY));
    }
  }

  /** Lock discards observations and prevents queued probes from starting. */
  clear(): void {
    this.#epoch++;
    this.#watchers.clear();
    this.#running.clear();
    this.data = {};
  }
}

export const reachability = new ReachabilityStore();
