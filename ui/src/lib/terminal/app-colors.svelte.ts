/**
 * The app's own window colours, for the "Match the app" terminal theme: the terminal takes the
 * window background, the text colour and the accent (cursor, selection), so it reads as part of
 * the window. Follows theme and accent changes.
 */
export const appColors = $state({ bg: '', fg: '', accent: '' });

function watch(): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const read = () => {
    const cs = getComputedStyle(root);
    appColors.bg = cs.getPropertyValue('--bg').trim();
    appColors.fg = cs.getPropertyValue('--text').trim();
    appColors.accent = cs.getPropertyValue('--accent').trim();
  };
  read();
  new MutationObserver(read).observe(root, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-accent']
  });
}

// Read once at load and on every theme/accent change, never from inside a derivation.
watch();
