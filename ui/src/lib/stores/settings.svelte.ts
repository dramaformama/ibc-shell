/** User settings: theme, language, remembered layout, recent flags. */
import { api } from '$lib/api';
import { getLocale, setLocale } from '$lib/paraglide/runtime.js';
import { ui } from './ui.svelte';

export type ThemeChoice = 'light' | 'dark' | 'system';

const LS_THEME = 'termlite.theme';

function readTheme(): ThemeChoice {
  try {
    const t = localStorage.getItem(LS_THEME);
    if (t === 'light' || t === 'dark' || t === 'system') return t;
  } catch {
    /* storage unavailable */
  }
  return 'light';
}

export function applyTheme(choice: ThemeChoice): void {
  const dark =
    choice === 'dark' ||
    (choice === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
}

class SettingsStore {
  theme = $state<ThemeChoice>(readTheme());
  recentFlags = $state.raw<string[]>([]);

  constructor() {
    if (typeof matchMedia !== 'undefined')
      matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (this.theme === 'system') applyTheme('system');
      });
  }

  get locale(): string {
    return getLocale();
  }

  async load(): Promise<void> {
    const [width, recent] = await Promise.all([
      api.settingsGet('ui.sidebar_width'),
      api.settingsGet('ui.flag_recent')
    ]);
    if (width.status === 'ok' && typeof width.data === 'number')
      ui.sidebarWidth = Math.min(420, Math.max(200, width.data));
    if (recent.status === 'ok' && Array.isArray(recent.data))
      this.recentFlags = recent.data.filter((v): v is string => typeof v === 'string').slice(0, 8);
  }

  setTheme(choice: ThemeChoice): void {
    this.theme = choice;
    try {
      localStorage.setItem(LS_THEME, choice);
    } catch {
      /* ignore */
    }
    applyTheme(choice);
  }

  toggleTheme(): void {
    const dark = document.documentElement.dataset.theme === 'dark';
    this.setTheme(dark ? 'light' : 'dark');
  }

  /** Changes the language; paraglide reloads the document. */
  setLanguage(locale: 'en' | 'ru'): void {
    void setLocale(locale);
  }

  saveSidebarWidth(): void {
    void api.settingsSet('ui.sidebar_width', Math.round(ui.sidebarWidth));
  }

  pushRecentFlag(code: string): void {
    this.recentFlags = [code, ...this.recentFlags.filter((c) => c !== code)].slice(0, 8);
    void api.settingsSet('ui.flag_recent', this.recentFlags);
  }
}

export const settings = new SettingsStore();
