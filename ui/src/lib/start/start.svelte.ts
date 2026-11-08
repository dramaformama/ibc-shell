/** "Getting started" checklist: add a server, add a key, make a backup. Progress lives in settings. */
import { api } from '$lib/api';
import { tree } from '$lib/stores/tree.svelte';

const KEY = 'onboarding.v1';

interface Saved {
  dismissed: boolean;
  backup: boolean;
}

class Start {
  dismissed = $state(false);
  backup = $state(false);
  /** A key (not just a password) has been added. */
  hasKey = $state(false);
  loaded = $state(false);

  hasServer = $derived(tree.data.hosts.length > 0);
  done = $derived([this.hasServer, this.hasKey, this.backup].filter(Boolean).length);
  /** The card shows until everything is done or the person hides it. */
  visible = $derived(this.loaded && !this.dismissed && this.done < 3);

  async load(): Promise<void> {
    const [s, ids] = await Promise.all([api.settingsGet(KEY), api.identityList()]);
    if (s.status === 'ok' && s.data && typeof s.data === 'object') {
      const v = s.data as Partial<Saved>;
      this.dismissed = v.dismissed === true;
      this.backup = v.backup === true;
    }
    if (ids.status === 'ok') this.hasKey = ids.data.some((i) => i.kind !== 'password');
    this.loaded = true;
  }

  clear(): void {
    this.dismissed = this.backup = this.hasKey = this.loaded = false;
  }

  #save(): void {
    const v: Saved = { dismissed: this.dismissed, backup: this.backup };
    void api.settingsSet(KEY, v as unknown as Parameters<typeof api.settingsSet>[1]);
  }

  dismiss(): void {
    this.dismissed = true;
    this.#save();
  }

  markBackup(): void {
    if (this.backup) return;
    this.backup = true;
    this.#save();
  }

  /** Re-reads the identity list (after a key was imported). */
  async refreshKeys(): Promise<void> {
    const ids = await api.identityList();
    if (ids.status === 'ok') this.hasKey = ids.data.some((i) => i.kind !== 'password');
  }
}

export const start = new Start();
