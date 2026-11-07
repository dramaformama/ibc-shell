/** Recently opened saved hosts (newest first), for quick reconnect from the overview / palette. */
import { api, type HostId } from '$lib/api';

const KEY = 'ui.recent_hosts';
const MAX = 8;

export interface RecentEntry {
  id: HostId;
  /** Unix ms of the last open. */
  at: number;
}

class RecentStore {
  items = $state.raw<RecentEntry[]>([]);

  async load(): Promise<void> {
    const r = await api.settingsGet(KEY);
    if (r.status !== 'ok' || !Array.isArray(r.data)) return;
    this.items = (r.data as unknown[])
      .filter(
        (v): v is RecentEntry =>
          !!v &&
          typeof v === 'object' &&
          typeof (v as RecentEntry).id === 'string' &&
          typeof (v as RecentEntry).at === 'number'
      )
      .slice(0, MAX);
  }

  push(id: HostId): void {
    this.items = [{ id, at: Date.now() }, ...this.items.filter((e) => e.id !== id)].slice(0, MAX);
    void api.settingsSet(
      KEY,
      this.items.map((e) => ({ id: e.id, at: e.at }))
    );
  }

  /** Drops hosts that no longer exist (deleted). */
  prune(exists: (id: HostId) => boolean): RecentEntry[] {
    return this.items.filter((e) => exists(e.id));
  }

  clear(): void {
    this.items = [];
    void api.settingsSet(KEY, []);
  }
}

export const recent = new RecentStore();
