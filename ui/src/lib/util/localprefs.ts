/**
 * Preferences the webview keeps in `localStorage` (theme, accent, layout, language). A backup
 * carries them as the setting `ui.local`, so a new computer looks the same.
 */
import { api, type Value } from '$lib/api';

const KEY = 'ui.local';
const EXTRA = ['PARAGLIDE_LOCALE'];
const MAX_VALUE = 8 * 1024;

const wanted = (k: string) => k.startsWith('ibc.') || EXTRA.includes(k);

/** The preferences kept in this webview. */
export function collectLocal(): Record<string, string> {
  const out: Record<string, string> = {};
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      const v = k ? localStorage.getItem(k) : null;
      if (k && v !== null && wanted(k) && v.length <= MAX_VALUE) out[k] = v;
    }
  } catch {
    /* storage unavailable */
  }
  return out;
}

/** Writes them into the settings store, so the next backup contains them. */
export async function saveLocal(): Promise<void> {
  await api.settingsSet(KEY, collectLocal() as Value);
}

/** After a restore: puts the preferences from the backup back into this webview. */
export async function restoreLocal(): Promise<void> {
  const r = await api.settingsGet(KEY);
  if (r.status !== 'ok' || !r.data || typeof r.data !== 'object' || Array.isArray(r.data)) return;
  try {
    for (const [k, v] of Object.entries(r.data)) {
      if (wanted(k) && typeof v === 'string' && v.length <= MAX_VALUE) localStorage.setItem(k, v);
    }
  } catch {
    /* storage unavailable */
  }
}
