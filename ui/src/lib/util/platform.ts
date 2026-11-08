/** Tiny platform helpers for shortcut labels. */
export const isApple =
  typeof navigator !== 'undefined' &&
  /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const PHYSICAL: Record<string, string> = { Comma: ',', Period: '.', Slash: '/', BracketLeft: '[' };

/**
 * The key of a shortcut, lower-cased, whatever the keyboard layout: with a Cyrillic layout `e.key`
 * is `л` for the physical K, so non-Latin characters fall back to the physical key.
 */
export function shortcutKey(e: KeyboardEvent): string {
  if (e.key.length !== 1 || e.key.charCodeAt(0) < 128) return e.key.toLowerCase();
  return e.code.startsWith('Key') ? e.code.slice(3).toLowerCase() : (PHYSICAL[e.code] ?? e.key);
}

/** Shortcut label: `⌘K` on Apple, `Ctrl+K` elsewhere. */
export const mod = (key: string): string => (isApple ? `⌘${key}` : `Ctrl+${key}`);
