/** Builds an xterm theme from 16 ANSI colours (docs/contracts/ui.md). */
import type { ITheme } from '@xterm/xterm';

export type Ansi = [
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

export function theme(
  bg: string,
  fg: string,
  cursor: string,
  selection: string,
  ansi: Ansi
): ITheme {
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
