/** Vault lock state. Passwords are passed straight to the API and never stored here. */
import { api, apiEvents, type VaultMode, type VaultState } from '$lib/api';
import { errorText } from '$lib/util/errors';

class VaultStore {
  state = $state<VaultState | 'Loading'>('Loading');
  mode = $state<VaultMode | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);
  #listening = false;

  async refresh(): Promise<void> {
    const r = await api.vaultStatus();
    if (r.status === 'ok') {
      this.state = r.data.state;
      this.mode = r.data.mode;
    }
    if (!this.#listening) {
      this.#listening = true;
      void apiEvents.listen('vaultLocked', () => (this.state = 'Locked'));
    }
  }

  async init(mode: VaultMode, password: string | null): Promise<boolean> {
    return this.#run(() => api.vaultInit(mode, password), mode);
  }

  async unlock(password: string | null): Promise<boolean> {
    return this.#run(() => api.vaultUnlock(password));
  }

  async lock(): Promise<void> {
    await api.vaultLock();
    this.state = 'Locked';
  }

  async #run(call: () => ReturnType<typeof api.vaultInit>, mode?: VaultMode): Promise<boolean> {
    this.busy = true;
    this.error = null;
    const r = await call();
    this.busy = false;
    if (r.status === 'error') {
      this.error = errorText(r.error);
      return false;
    }
    if (mode) this.mode = mode;
    this.state = 'Unlocked';
    return true;
  }
}

export const vault = new VaultStore();
