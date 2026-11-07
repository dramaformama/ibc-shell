/** Tiny platform helpers for shortcut labels. */
export const isApple =
  typeof navigator !== 'undefined' &&
  /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

/** Shortcut label: `⌘K` on Apple, `Ctrl+K` elsewhere. */
export const mod = (key: string): string => (isApple ? `⌘${key}` : `Ctrl+${key}`);
