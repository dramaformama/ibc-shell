/** Terminal preferences shared by every terminal tab: theme, font size (zoom) and scrollback. */
import { api } from '$lib/api';

export const FONT_MIN = 9;
export const FONT_MAX = 28;
export const DEFAULT_FONT = 13;

/** Scrollback sizes offered in Settings. */
export const SCROLLBACK_CHOICES = [1_000, 5_000, 10_000, 50_000, 100_000] as const;

const clamp = (n: number) => Math.min(FONT_MAX, Math.max(FONT_MIN, Math.round(n)));

class TerminalPrefs {
  fontSize = $state(DEFAULT_FONT);
  scrollback = $state(10_000);
  /** Colour IPs, URLs, errors… in plain server output (highlight.ts). */
  highlight = $state(true);
  /** Notify when a command in a background terminal finishes or rings the bell. */
  notify = $state(true);
  /** Reconnect by itself after the network or the server dropped the connection. */
  autoReconnect = $state(true);
  /** App-wide terminal theme; `null` follows the light / dark appearance. */
  theme = $state<string | null>(null);
  #loaded: Promise<void> | null = null;

  /** Read persisted values once. */
  load(): Promise<void> {
    this.#loaded ??= (async () => {
      const [f, s, h, t, n, a] = await Promise.all([
        api.settingsGet('term.font_size'),
        api.settingsGet('term.scrollback'),
        api.settingsGet('term.highlight'),
        api.settingsGet('term.theme'),
        api.settingsGet('term.notify'),
        api.settingsGet('term.auto_reconnect')
      ]);
      if (a.status === 'ok' && a.data === false) this.autoReconnect = false;
      if (n.status === 'ok' && n.data === false) this.notify = false;
      if (t.status === 'ok' && typeof t.data === 'string') this.theme = t.data;
      if (h.status === 'ok' && h.data === false) this.highlight = false;
      if (f.status === 'ok' && typeof f.data === 'number') this.fontSize = clamp(f.data);
      if (s.status === 'ok' && typeof s.data === 'number')
        this.scrollback = Math.min(100_000, Math.max(500, Math.round(s.data)));
    })();
    return this.#loaded;
  }

  setHighlight(on: boolean): void {
    this.highlight = on;
    void api.settingsSet('term.highlight', on);
  }

  setNotify(on: boolean): void {
    this.notify = on;
    void api.settingsSet('term.notify', on);
  }

  setAutoReconnect(on: boolean): void {
    this.autoReconnect = on;
    void api.settingsSet('term.auto_reconnect', on);
  }

  setTheme(id: string | null): void {
    this.theme = id;
    void api.settingsSet('term.theme', id);
  }

  setScrollback(lines: number): void {
    this.scrollback = lines;
    void api.settingsSet('term.scrollback', lines);
  }

  /** ⌘+ / ⌘- (delta) or ⌘0 (reset). */
  zoom(delta: number | 'reset'): void {
    this.fontSize = delta === 'reset' ? DEFAULT_FONT : clamp(this.fontSize + delta);
    void api.settingsSet('term.font_size', this.fontSize);
  }
}

export const terminalPrefs = new TerminalPrefs();
