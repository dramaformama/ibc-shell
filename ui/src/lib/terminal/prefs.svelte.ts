/** Terminal preferences shared by every terminal tab: font size (zoom) and scrollback. */
import { api } from '$lib/api';

const MIN = 9;
const MAX = 28;
export const DEFAULT_FONT = 13;

const clamp = (n: number) => Math.min(MAX, Math.max(MIN, Math.round(n)));

class TerminalPrefs {
  fontSize = $state(DEFAULT_FONT);
  scrollback = $state(10_000);
  /** Colour IPs, URLs, errors… in plain server output (highlight.ts). */
  highlight = $state(true);
  #loaded: Promise<void> | null = null;

  /** Read persisted values once. */
  load(): Promise<void> {
    this.#loaded ??= (async () => {
      const [f, s, h] = await Promise.all([
        api.settingsGet('term.font_size'),
        api.settingsGet('term.scrollback'),
        api.settingsGet('term.highlight')
      ]);
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

  /** ⌘+ / ⌘- (delta) or ⌘0 (reset). */
  zoom(delta: number | 'reset'): void {
    this.fontSize = delta === 'reset' ? DEFAULT_FONT : clamp(this.fontSize + delta);
    void api.settingsSet('term.font_size', this.fontSize);
  }
}

export const terminalPrefs = new TerminalPrefs();
