/** User settings: theme, language, remembered layout, recent flags. */
import { api } from '$lib/api';
import { getLocale, setLocale } from '$lib/paraglide/runtime.js';
import { checkForUpdate, UPDATE_CHECK_KEY, type UpdateInfo } from '$lib/util/update';
import { AUTOLOCK_DEFAULT, AUTOLOCK_KEY, parseAutolock } from '$lib/util/autolock';
import { ui } from './ui.svelte';

export type ThemeChoice = 'light' | 'dark' | 'system';

const LS_THEME = 'ibc.theme';
const LS_ACCENT = 'ibc.accent';
const PROBE_KEY = 'fleet.probe';
const TRAY_KEY = 'ui.tray';

/** Apple system accent colours offered in Settings (CSS in app.css). */
export const ACCENTS = [
  'blue',
  'purple',
  'pink',
  'red',
  'orange',
  'green',
  'teal',
  'indigo',
  'graphite'
] as const;
export type Accent = (typeof ACCENTS)[number];

function readAccent(): Accent {
  try {
    const a = localStorage.getItem(LS_ACCENT);
    if (a && (ACCENTS as readonly string[]).includes(a)) return a as Accent;
  } catch {
    /* storage unavailable */
  }
  return 'blue';
}

function readTheme(): ThemeChoice {
  try {
    const t = localStorage.getItem(LS_THEME);
    if (t === 'light' || t === 'dark' || t === 'system') return t;
  } catch {
    /* storage unavailable */
  }
  return 'light';
}

export type Density = 'roomy' | 'compact';

function readDensity(): Density {
  try {
    if (localStorage.getItem('ibc.density') === 'compact') return 'compact';
  } catch {
    /* storage unavailable */
  }
  return 'roomy';
}

export function applyDensity(d: Density): void {
  if (d === 'compact') document.documentElement.dataset.density = 'compact';
  else delete document.documentElement.dataset.density;
}

if (typeof document !== 'undefined') applyDensity(readDensity());

export function applyTheme(choice: ThemeChoice): void {
  const dark =
    choice === 'dark' ||
    (choice === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
}

/** A remembered on/off switch that must be known before unlock (default on). */
function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) !== '0';
  } catch {
    return true;
  }
}

class SettingsStore {
  theme = $state<ThemeChoice>(readTheme());
  accent = $state<Accent>(readAccent());
  /** Row height of long lists (servers, folders, processes). */
  density = $state<Density>(readDensity());
  recentFlags = $state.raw<string[]>([]);
  /** Idle minutes before the vault locks (0 = never). */
  autolockMinutes = $state(AUTOLOCK_DEFAULT);
  /** Whether to look for a newer release on GitHub at start. */
  updateCheck = $state(true);
  /** TCP availability checks of saved servers on the overview (ADR-010). */
  probe = $state(true);
  /** Menu-bar icon with a server summary (ADR-013). */
  tray = $state(readFlag('ibc.tray'));
  /** A newer release, once found. */
  update = $state.raw<UpdateInfo | null>(null);

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
    const [width, recent, autolock, upd, probe, tray] = await Promise.all([
      api.settingsGet('ui.sidebar_width'),
      api.settingsGet('ui.flag_recent'),
      api.settingsGet(AUTOLOCK_KEY),
      api.settingsGet(UPDATE_CHECK_KEY),
      api.settingsGet(PROBE_KEY),
      api.settingsGet(TRAY_KEY)
    ]);
    this.tray = !(tray.status === 'ok' && tray.data === false);
    if (probe.status === 'ok' && probe.data === false) this.probe = false;
    if (upd.status === 'ok' && upd.data === false) this.updateCheck = false;
    if (autolock.status === 'ok')
      this.autolockMinutes = parseAutolock(autolock.data ?? AUTOLOCK_DEFAULT);
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

  setDensity(d: Density): void {
    this.density = d;
    try {
      localStorage.setItem('ibc.density', d);
    } catch {
      /* ignore */
    }
    applyDensity(d);
  }

  setAccent(a: Accent): void {
    this.accent = a;
    try {
      localStorage.setItem(LS_ACCENT, a);
    } catch {
      /* ignore */
    }
    if (a === 'blue') delete document.documentElement.dataset.accent;
    else document.documentElement.dataset.accent = a;
  }

  toggleTheme(): void {
    const dark = document.documentElement.dataset.theme === 'dark';
    this.setTheme(dark ? 'light' : 'dark');
  }

  /** Changes the language; paraglide reloads the document. */
  setLanguage(locale: 'en' | 'ru'): void {
    void setLocale(locale);
  }

  setUpdateCheck(on: boolean): void {
    this.updateCheck = on;
    void api.settingsSet(UPDATE_CHECK_KEY, on);
    if (on) void this.checkUpdate();
    else this.update = null;
  }

  /** Looks for a newer release (no-op when disabled or already found). */
  async checkUpdate(): Promise<void> {
    if (!this.updateCheck || this.update) return;
    const info = await api.appInfo();
    if (info.status !== 'ok') return;
    this.update = await checkForUpdate(info.data.version);
  }

  /** Re-checks every 6 h and when the window regains focus (at most once per hour). */
  watchUpdates(): () => void {
    let last = Date.now();
    const run = () => {
      last = Date.now();
      void this.checkUpdate();
    };
    const timer = setInterval(run, 6 * 3600_000);
    const onFocus = () => {
      if (Date.now() - last > 3600_000) run();
    };
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(timer);
      window.removeEventListener('focus', onFocus);
    };
  }

  setTray(on: boolean): void {
    this.tray = on;
    try {
      localStorage.setItem('ibc.tray', on ? '1' : '0');
    } catch {
      /* ignore */
    }
    void api.settingsSet(TRAY_KEY, on);
  }

  setProbe(on: boolean): void {
    this.probe = on;
    void api.settingsSet(PROBE_KEY, on);
  }

  setAutolock(minutes: number): void {
    this.autolockMinutes = parseAutolock(minutes);
    void api.settingsSet(AUTOLOCK_KEY, this.autolockMinutes);
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
