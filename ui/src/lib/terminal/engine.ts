/** Lazy xterm setup: core + addons are code-split, WebGL renderer with DOM fallback. */
import type { ITheme, Terminal } from '@xterm/xterm';
import type { FitAddon } from '@xterm/addon-fit';
import type { SearchAddon } from '@xterm/addon-search';

export interface Engine {
  term: Terminal;
  fit: FitAddon;
  search: SearchAddon;
  /** Current renderer; becomes `dom` after a lost WebGL context. */
  renderer: () => 'webgl' | 'dom';
  dispose: () => void;
}

export interface EngineOptions {
  theme: ITheme;
  fontSize: number;
  scrollback: number;
  /** OSC 8 and plain-text links; the caller confirms and validates. */
  onLink: (url: string) => void;
}

export const MONO =
  "'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace";

/** Wait briefly for the bundled mono face so the first cell measurement is right. */
async function fontReady(size: number): Promise<void> {
  try {
    await Promise.race([
      document.fonts.load(`${size}px "JetBrains Mono"`),
      new Promise((r) => setTimeout(r, 300))
    ]);
  } catch {
    /* measurement falls back to the next family */
  }
}

/** Create and open a terminal inside `el`. OSC 52 stays off (no clipboard addon). */
export async function createEngine(el: HTMLElement, o: EngineOptions): Promise<Engine> {
  const [{ Terminal }, { FitAddon }, { SearchAddon }, { WebLinksAddon }, { Unicode11Addon }] =
    await Promise.all([
      import('@xterm/xterm'),
      import('@xterm/addon-fit'),
      import('@xterm/addon-search'),
      import('@xterm/addon-web-links'),
      import('@xterm/addon-unicode11'),
      import('@xterm/xterm/css/xterm.css'),
      fontReady(o.fontSize)
    ]);

  const term = new Terminal({
    allowProposedApi: true,
    fontFamily: MONO,
    fontSize: o.fontSize,
    lineHeight: 1.2,
    scrollback: o.scrollback,
    theme: o.theme,
    // Programs assume a dark screen and print white; xterm darkens such cells just enough to read.
    minimumContrastRatio: 4.5,
    cursorBlink: false,
    cursorStyle: 'bar',
    cursorWidth: 2,
    macOptionIsMeta: false,
    smoothScrollDuration: 0,
    linkHandler: { activate: (_e, text) => o.onLink(text), allowNonHttpProtocols: false }
  });
  const fit = new FitAddon();
  const search = new SearchAddon();
  term.loadAddon(fit);
  term.loadAddon(search);
  term.loadAddon(new WebLinksAddon((_e, uri) => o.onLink(uri)));
  term.loadAddon(new Unicode11Addon());
  term.unicode.activeVersion = '11';
  term.open(el);

  let mode: 'webgl' | 'dom' = 'dom';
  try {
    const { WebglAddon } = await import('@xterm/addon-webgl');
    const gl = new WebglAddon();
    gl.onContextLoss(() => {
      gl.dispose();
      mode = 'dom';
    });
    term.loadAddon(gl);
    mode = 'webgl';
  } catch {
    mode = 'dom';
  }

  return {
    term,
    fit,
    search,
    renderer: () => mode,
    dispose: () => term.dispose()
  };
}
