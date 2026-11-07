/**
 * Mock: vault, app commands and the global event bus.
 * Vault starts `Locked` (MasterPassword). Password `termlite` unlocks; anything else →
 * `WrongPassword`. `?vault=new` in the URL starts `Uninitialized` (onboarding).
 */
import type { ApiEvents, Commands, EventPayloads, VaultMode, VaultState } from '../types';
import { delay, err, ok } from './state';

const MOCK_PASSWORD = 'termlite';

const initialUninit =
  typeof location !== 'undefined' && new URLSearchParams(location.search).get('vault') === 'new';
let state: VaultState = initialUninit ? 'Uninitialized' : 'Locked';
let mode: VaultMode | null = initialUninit ? null : 'MasterPassword';
let password = MOCK_PASSWORD;

export const vaultIsUnlocked = (): boolean => state === 'Unlocked';

// ---- event bus ----
type Listener = (payload: unknown) => void;
const listeners = new Map<keyof EventPayloads, Set<Listener>>();

export function emit<K extends keyof EventPayloads>(name: K, payload: EventPayloads[K]): void {
  listeners.get(name)?.forEach((cb) => cb(payload));
}

export const mockEvents: ApiEvents = {
  async listen(name, cb) {
    const set = listeners.get(name) ?? new Set<Listener>();
    listeners.set(name, set);
    const l = cb as Listener;
    set.add(l);
    return () => set.delete(l);
  }
};

// ---- vault ----
export const vaultStatus: Commands['vaultStatus'] = async () =>
  ok({ state, mode, biometric_available: false });

export const vaultInit: Commands['vaultInit'] = async (newMode, pw) => {
  await delay(300);
  if (state !== 'Uninitialized') return err({ code: 'Validation', detail: 'already initialized' });
  if (newMode === 'MasterPassword' && !pw)
    return err({ code: 'Validation', detail: 'password required' });
  mode = newMode;
  password = pw ?? MOCK_PASSWORD;
  state = 'Unlocked';
  return ok(null);
};

export const vaultUnlock: Commands['vaultUnlock'] = async (pw) => {
  await delay(300); // Argon2 feel
  if (state === 'Uninitialized') return err({ code: 'Validation', detail: 'not initialized' });
  if (mode === 'MasterPassword' && pw !== password) return err({ code: 'WrongPassword' });
  state = 'Unlocked';
  return ok(null);
};

export const vaultLock: Commands['vaultLock'] = async () => {
  if (state === 'Unlocked') {
    state = 'Locked';
    emit('vaultLocked', null);
  }
  return ok(null);
};

export const vaultChangePassword: Commands['vaultChangePassword'] = async (
  oldPassword,
  newPassword
) => {
  await delay(300);
  if (state !== 'Unlocked') return err({ code: 'VaultLocked' });
  if (oldPassword !== password) return err({ code: 'WrongPassword' });
  password = newPassword;
  return ok(null);
};

export const vaultExport: Commands['vaultExport'] = async (_path, pw) => {
  await delay(500);
  if (state !== 'Unlocked') return err({ code: 'VaultLocked' });
  if (pw.length < 10)
    return err({ code: 'Validation', detail: 'backup password must be at least 10 characters' });
  return ok(null);
};

export const vaultImport: Commands['vaultImport'] = async (_path, pw) => {
  await delay(500);
  if (state !== 'Unlocked') return err({ code: 'VaultLocked' });
  if (pw.length < 10) return err({ code: 'WrongPassword' });
  emit('treeChanged', null);
  return ok({
    folders: 0,
    hosts: 0,
    identities: 0,
    tags: 0,
    snippets: 0,
    forwards: 0,
    known_hosts: 0,
    skipped: 0
  });
};

// ---- app ----
export const appInfo: Commands['appInfo'] = async () =>
  ok({ version: '0.1.0-mock', platform: 'macos', arch: 'aarch64' });

export const openExternal: Commands['openExternal'] = async (url) => {
  if (!/^https?:\/\//i.test(url))
    return err({ code: 'Validation', detail: 'only http/https URLs' });
  window.open(url, '_blank', 'noopener,noreferrer');
  return ok(null);
};
