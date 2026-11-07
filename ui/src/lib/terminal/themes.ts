/** Terminal themes (docs/contracts/ui.md): 16 ANSI + foreground / background / cursor / selection. */
import type { ITheme } from '@xterm/xterm';

type Ansi = [
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string
];

function theme(bg: string, fg: string, cursor: string, selection: string, ansi: Ansi): ITheme {
  const [black, red, green, yellow, blue, magenta, cyan, white] = ansi;
  return {
    background: bg,
    foreground: fg,
    cursor,
    cursorAccent: bg,
    selectionBackground: selection,
    black,
    red,
    green,
    yellow,
    blue,
    magenta,
    cyan,
    white,
    brightBlack: ansi[8],
    brightRed: ansi[9],
    brightGreen: ansi[10],
    brightYellow: ansi[11],
    brightBlue: ansi[12],
    brightMagenta: ansi[13],
    brightCyan: ansi[14],
    brightWhite: ansi[15]
  };
}

export const TERMINAL_THEMES: Record<string, ITheme> = {
  'ibc-light': theme('#ffffff', '#16181d', '#3a66d6', 'rgba(58,102,214,0.22)', [
    '#16181d',
    '#c93a3a',
    '#1b8a55',
    '#a8680a',
    '#3a66d6',
    '#8a4fc2',
    '#12809a',
    '#c9ccd2',
    '#6b7280',
    '#e05252',
    '#1f9d5f',
    '#b97709',
    '#5479e0',
    '#9d63d6',
    '#1a97b5',
    '#f2f3f5'
  ]),
  'ibc-dark': theme('#0e0f12', '#e6e8eb', '#5b8cff', 'rgba(91,140,255,0.30)', [
    '#1d2026',
    '#f06161',
    '#3ccb7f',
    '#f5b547',
    '#5b8cff',
    '#b58cf0',
    '#4cc9e0',
    '#c9ccd2',
    '#596070',
    '#ff7b7b',
    '#5fe29a',
    '#ffc96a',
    '#7ea3ff',
    '#c9a6ff',
    '#7bdcf0',
    '#f2f3f5'
  ]),
  'solarized-light': theme('#fdf6e3', '#586e75', '#586e75', 'rgba(38,139,210,0.20)', [
    '#073642',
    '#dc322f',
    '#859900',
    '#b58900',
    '#268bd2',
    '#d33682',
    '#2aa198',
    '#eee8d5',
    '#586e75',
    '#cb4b16',
    '#657b83',
    '#839496',
    '#93a1a1',
    '#6c71c4',
    '#7aa6a1',
    '#fdf6e3'
  ]),
  'solarized-dark': theme('#002b36', '#93a1a1', '#93a1a1', 'rgba(38,139,210,0.30)', [
    '#073642',
    '#dc322f',
    '#859900',
    '#b58900',
    '#268bd2',
    '#d33682',
    '#2aa198',
    '#eee8d5',
    '#586e75',
    '#cb4b16',
    '#7a8f94',
    '#839496',
    '#93a1a1',
    '#6c71c4',
    '#7aa6a1',
    '#fdf6e3'
  ]),
  night: theme('#1b1a26', '#d8d6e8', '#ff79c6', 'rgba(189,147,249,0.30)', [
    '#2a2838',
    '#ff6b81',
    '#6ee7a0',
    '#f3d675',
    '#8f9bff',
    '#c792ff',
    '#6fd8e8',
    '#d8d6e8',
    '#5c5a73',
    '#ff8fa0',
    '#8af0b5',
    '#f8e49a',
    '#aab3ff',
    '#dab0ff',
    '#92e6f2',
    '#fbfaff'
  ]),
  frost: theme('#232831', '#d5dbe5', '#88c0d0', 'rgba(136,192,208,0.28)', [
    '#2e3440',
    '#d57780',
    '#a3be8c',
    '#e3c78a',
    '#7aa2c9',
    '#b48ead',
    '#88c0d0',
    '#d5dbe5',
    '#4c566a',
    '#e08e96',
    '#b5cfa0',
    '#eed49f',
    '#93b4d6',
    '#c6a3c0',
    '#9fd0de',
    '#eceff4'
  ])
};

/** Resolve a stored theme id; `null` follows the app appearance. */
export function resolveTheme(id: string | null | undefined, appDark: boolean): ITheme {
  const found = id ? TERMINAL_THEMES[id] : undefined;
  return found ?? TERMINAL_THEMES[appDark ? 'ibc-dark' : 'ibc-light']!;
}
