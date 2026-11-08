/** Every keyboard shortcut of the app, grouped for the "Keyboard shortcuts" window. */
import * as m from '$lib/paraglide/messages.js';
import { isApple, mod } from '$lib/util/platform';

export interface Shortcut {
  label: string;
  /** One key cap per element (a chord is shown as several caps). */
  keys: string[];
}
export interface ShortcutGroup {
  title: string;
  items: Shortcut[];
}

/** Splits `⌘K` / `Ctrl+Shift+F` into caps. */
function caps(s: string): string[] {
  return isApple && s.startsWith('⌘') ? ['⌘', s.slice(1)] : s.split('+');
}
const app = (k: string) => caps(mod(k));
/** In the terminal, Windows / Linux use Ctrl+Shift so that plain Ctrl keeps working in the shell. */
const term = (k: string) => (isApple ? ['⌘', k] : ['Ctrl', 'Shift', k]);

export function shortcutGroups(): ShortcutGroup[] {
  return [
    {
      title: m.keys_group_app(),
      items: [
        { label: m.keys_palette(), keys: app('K') },
        { label: m.keys_sidebar(), keys: app('B') },
        { label: m.keys_settings(), keys: app(',') },
        { label: m.keys_lock(), keys: app('L') },
        { label: m.keys_help(), keys: ['?'] }
      ]
    },
    {
      title: m.keys_group_tabs(),
      items: [
        { label: m.keys_new_tab(), keys: app('T') },
        { label: m.menu_connect_another(), keys: term('T') },
        { label: m.keys_close_tab(), keys: app('W') },
        { label: m.keys_switch_server(), keys: [...app('1'), '–', '9'] },
        { label: m.keys_next_tab(), keys: ['Ctrl', 'Tab'] }
      ]
    },
    {
      title: m.keys_group_terminal(),
      items: [
        { label: m.keys_find(), keys: term('F') },
        { label: m.keys_copy(), keys: term('C') },
        ...(isApple ? [] : [{ label: m.keys_paste(), keys: term('V') }]),
        { label: m.keys_zoom_in(), keys: [...term('+')] },
        { label: m.keys_zoom_out(), keys: [...term('-')] },
        { label: m.keys_zoom_reset(), keys: [...term('0')] }
      ]
    },
    {
      title: m.keys_group_files(),
      items: [
        { label: m.keys_open(), keys: ['Enter'] },
        { label: m.keys_rename(), keys: ['F2'] },
        { label: m.keys_delete(), keys: ['Delete'] },
        { label: m.keys_select_all(), keys: app('A') },
        { label: m.keys_up(), keys: ['Backspace'] }
      ]
    }
  ];
}
