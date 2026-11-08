/** Form model shared by the host and folder editors: inheritable fields as strings ('' = inherit). */
import type { FolderDefaults } from '$lib/api';

export interface DefaultsForm {
  username: string;
  port: string;
  identity: string;
  jump: string;
  startup: string;
  keepalive: string;
  theme: string;
}

/** Shape common to `Host` and `FolderDefaults` (only the inheritable columns). */
export interface DefaultsSource {
  username: string | null;
  port: number | null;
  identity_id: string | null;
  jump_host_id: string | null;
  startup_cmd: string | null;
  keepalive_s: number | null;
  term_theme: string | null;
}

export const emptyDefaults = (): DefaultsForm => ({
  username: '',
  port: '',
  identity: '',
  jump: '',
  startup: '',
  keepalive: '',
  theme: ''
});

export function formFrom(src: DefaultsSource | FolderDefaults): DefaultsForm {
  return {
    username: src.username ?? '',
    port: src.port?.toString() ?? '',
    identity: src.identity_id ?? '',
    jump: src.jump_host_id ?? '',
    startup: src.startup_cmd ?? '',
    keepalive: src.keepalive_s?.toString() ?? '',
    theme: src.term_theme ?? ''
  };
}

const num = (s: string): number | null => {
  const n = Number.parseInt(s, 10);
  return Number.isFinite(n) && n > 0 ? n : null;
};

/** Full value set (create): '' → null. */
export function toValues(f: DefaultsForm): DefaultsSource {
  return {
    username: f.username.trim() || null,
    port: num(f.port),
    identity_id: f.identity || null,
    jump_host_id: f.jump || null,
    startup_cmd: f.startup.trim() || null,
    keepalive_s: num(f.keepalive),
    term_theme: f.theme || null
  };
}

/** Only fields that changed (update): cleared fields become explicit `null` (= inherit again). */
export function toPatch(before: DefaultsForm, after: DefaultsForm): Partial<DefaultsSource> {
  const a = toValues(before);
  const b = toValues(after);
  const out: Record<string, unknown> = {};
  for (const k of Object.keys(b) as (keyof DefaultsSource)[]) if (a[k] !== b[k]) out[k] = b[k];
  return out as Partial<DefaultsSource>;
}

/** Terminal themes shipped (ui.md). Names are proper nouns, not translated. */
export const TERM_THEMES = [
  { id: 'ibc-dark', name: 'iBC Shell Dark' },
  { id: 'ibc-light', name: 'iBC Shell Light' },
  { id: 'solarized-dark', name: 'Solarized Dark' },
  { id: 'solarized-light', name: 'Solarized Light' },
  { id: 'night', name: 'Night' },
  { id: 'frost', name: 'Frost' },
  { id: 'dracula', name: 'Dracula' },
  { id: 'one-dark', name: 'One Dark' },
  { id: 'tokyo-night', name: 'Tokyo Night' },
  { id: 'catppuccin-mocha', name: 'Catppuccin Mocha' },
  { id: 'catppuccin-latte', name: 'Catppuccin Latte' },
  { id: 'gruvbox-dark', name: 'Gruvbox Dark' },
  { id: 'gruvbox-light', name: 'Gruvbox Light' },
  { id: 'monokai', name: 'Monokai' },
  { id: 'github-dark', name: 'GitHub Dark' }
] as const;
