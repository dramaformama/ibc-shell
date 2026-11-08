/**
 * Docker resources per session (ADR-016): compose projects, images, volumes, networks, events.
 * Unlike containers (always polled while connected) these are read only while their tab is
 * open and the window is visible; events come as a short sliding window, never as a stream.
 */
import {
  api,
  type DockerCompose,
  type DockerEvents,
  type DockerImages,
  type DockerNetworks,
  type DockerVolumes,
  type Result,
  type SessionId
} from '$lib/api';
import { hidden, whenVisible } from '$lib/util/visible';

export type ResKind = 'compose' | 'images' | 'volumes' | 'networks' | 'events';

interface Shapes {
  compose: DockerCompose;
  images: DockerImages;
  volumes: DockerVolumes;
  networks: DockerNetworks;
  events: DockerEvents;
}

/** Poll period per kind, ms. Events are cheap and are what the user watches live. */
export const EVERY: Record<ResKind, number> = {
  compose: 20_000,
  images: 60_000,
  volumes: 60_000,
  networks: 60_000,
  events: 5_000
};
/** How far back the events list reaches, seconds. */
export const EVENT_WINDOW = 3600;

const call = (kind: ResKind, sid: SessionId): Promise<Result<Shapes[ResKind]>> =>
  kind === 'compose'
    ? api.dockerCompose(sid)
    : kind === 'images'
      ? api.dockerImages(sid)
      : kind === 'volumes'
        ? api.dockerVolumes(sid)
        : kind === 'networks'
          ? api.dockerNetworks(sid)
          : api.dockerEvents(sid, EVENT_WINDOW);

class DockerResStore {
  compose = $state<Record<string, DockerCompose>>({});
  images = $state<Record<string, DockerImages>>({});
  volumes = $state<Record<string, DockerVolumes>>({});
  networks = $state<Record<string, DockerNetworks>>({});
  events = $state<Record<string, DockerEvents>>({});
  #watchers = new Map<string, number>();
  #running = new Set<string>();
  #epoch = 0;

  /** Starts (or joins) polling of one kind for a session; returns the stop function. */
  watch(kind: ResKind, sid: SessionId): () => void {
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

  async #loop(kind: ResKind, sid: SessionId, key: string, epoch: number): Promise<void> {
    const live = () => this.#watchers.has(key) && epoch === this.#epoch;
    while (live()) {
      if (hidden()) {
        await whenVisible();
        if (!live()) break;
      }
      const r = await call(kind, sid).catch(() => null);
      if (!live()) break;
      if (r?.status === 'ok') {
        (this[kind] as Record<string, unknown>)[sid] = r.data;
        // Missing / no access / daemon down do not change by themselves.
        if (r.data.access !== 'Ok') break;
      } else if (r?.status === 'error' && r.error.code === 'NotFound') break;
      await new Promise((res) => setTimeout(res, EVERY[kind]));
    }
  }

  /** Lock: forget everything and stop the loops. */
  clear(): void {
    this.#epoch++;
    this.#watchers.clear();
    this.compose = {};
    this.images = {};
    this.volumes = {};
    this.networks = {};
    this.events = {};
  }
}

export const dockerRes = new DockerResStore();
