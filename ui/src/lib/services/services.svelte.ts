/**
 * Docker and certificate reports per session (ADR-012), shared by the Status page, the side
 * panel and the overview tabs: one polling loop per session however many views watch it.
 */
import { api, type CertReport, type DockerReport, type SessionId } from '$lib/api';
import { hidden, whenVisible } from '$lib/util/visible';

const DOCKER_EVERY = 20_000;
/** Certificates change rarely. */
const CERTS_EVERY = 600_000;

type Kind = 'docker' | 'certs';

class ServicesStore {
  docker = $state<Record<string, DockerReport>>({});
  certs = $state<Record<string, CertReport>>({});
  #watchers = new Map<string, number>();
  #running = new Set<string>();
  #epoch = 0;

  /** Starts (or joins) polling of `kind` for a session; returns the stop function. */
  watch(kind: Kind, sid: SessionId): () => void {
    const key = `${kind}|${sid}`;
    this.#watchers.set(key, (this.#watchers.get(key) ?? 0) + 1);
    if (!this.#running.has(key)) {
      this.#running.add(key);
      void this.#loop(kind, sid, key, this.#epoch).finally(() => this.#running.delete(key));
    }
    let stopped = false;
    return () => {
      if (stopped) return;
      stopped = true;
      const left = (this.#watchers.get(key) ?? 1) - 1;
      if (left <= 0) this.#watchers.delete(key);
      else this.#watchers.set(key, left);
    };
  }

  /** Re-reads the container list now (after an action). */
  async refreshDocker(sid: SessionId): Promise<void> {
    const r = await api.dockerList(sid).catch(() => null);
    if (r?.status === 'ok') this.docker[sid] = r.data;
  }

  async #loop(kind: Kind, sid: SessionId, key: string, epoch: number): Promise<void> {
    const live = () => this.#watchers.has(key) && epoch === this.#epoch;
    while (live()) {
      if (hidden()) {
        await whenVisible();
        if (!live()) break;
      }
      if (kind === 'docker') {
        const r = await api.dockerList(sid).catch(() => null);
        if (!live()) break;
        if (r?.status === 'ok') {
          this.docker[sid] = r.data;
          // Missing / no access / daemon down do not change by themselves.
          if (r.data.access !== 'Ok') break;
        } else if (r?.status === 'error' && r.error.code === 'NotFound') break;
      } else {
        const r = await api.certList(sid).catch(() => null);
        if (!live()) break;
        if (r?.status === 'ok') this.certs[sid] = r.data;
        else if (r?.status === 'error' && r.error.code === 'NotFound') break;
      }
      await new Promise((res) => setTimeout(res, kind === 'docker' ? DOCKER_EVERY : CERTS_EVERY));
    }
  }

  /** Lock: forget everything and stop the loops. */
  clear(): void {
    this.#epoch++;
    this.#watchers.clear();
    this.docker = {};
    this.certs = {};
  }
}

export const services = new ServicesStore();
