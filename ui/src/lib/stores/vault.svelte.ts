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

  /** Only a password vault can be locked; a keychain vault just reopens itself (ADR-014). */
  get canLock(): boolean {
    return this.mode === 'MasterPassword';
  }

  /** Keychain flow without a prompt: creates the vault on first start, opens it afterwards. */
  async auto(): Promise<boolean> {
    if (this.busy) return false;
    if (this.state === 'Uninitialized') return this.init('OsKeychain', null);
    if (this.state === 'Locked' && this.mode === 'OsKeychain') return this.unlock(null);
    return false;
  }

  /** Last password entry of a legacy vault: moves it to the keychain and opens it. */
  async removePassword(password: string): Promise<boolean> {
    const ok = await this.#run(() => api.vaultRemovePassword(password));
    if (ok) this.mode = 'OsKeychain';
    return ok;
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
