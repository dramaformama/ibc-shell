/**
 * Watches the output of a terminal the user is not looking at and decides when to tell them:
 * - `bell`: the program rang the bell (BEL outside an OSC string);
 * - `done`: the output ends with a shell prompt after the tab was hidden for a while, i.e. a
 *   long command finished. One notice per hidden stint, so `watch` / logs do not spam.
 * Heuristic by design: the server's shell is not ours to instrument.
 */

/** How long the tab must have been hidden before a returning prompt counts as "done". */
export const MIN_HIDDEN_MS = 5_000;
const TAIL = 256;

// OSC strings (titles, OSC 7) end with BEL themselves; CSI / other escapes carry no text.
// eslint-disable-next-line no-control-regex -- matching terminal escape sequences is the point
const OSC = /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g;
// eslint-disable-next-line no-control-regex -- same
const ESC = /\x1b(?:\[[0-?]*[ -/]*[@-~]|[@-Z\\-_])/g;
/**
 * Typical prompt endings: `…:~$ `, `…# ` anywhere; `% `, `> `, `❯ `, `➜ ` only after a space
 * or at the line start, so `45%` in a progress line is not a prompt.
 */
const PROMPT = /(?:[$#]|(?:^|[^\S\n])[%>❯➜›λ])[^\S\n]?$/;

export type Notice = 'bell' | 'done' | null;

export class ActivityWatch {
  #hiddenAt: number | null = null;
  #told = false;
  #tail = '';

  /** The tab was hidden (`at` = now) or shown (`null`). */
  setHidden(at: number | null): void {
    this.#hiddenAt = at;
    this.#told = false;
    this.#tail = '';
  }

  get hidden(): boolean {
    return this.#hiddenAt !== null;
  }

  /** Decoded output while hidden; returns what to tell the user, if anything. */
  feed(text: string, now: number): Notice {
    if (this.#hiddenAt === null || this.#told) return null;
    const plain = text.replace(OSC, '');
    if (plain.includes('\x07')) {
      this.#told = true;
      return 'bell';
    }
    const visible = plain.replace(ESC, '').replace(/\r/g, '');
    this.#tail = (this.#tail + visible).slice(-TAIL);
    const lastLine = this.#tail.slice(this.#tail.lastIndexOf('\n') + 1);
    if (now - this.#hiddenAt >= MIN_HIDDEN_MS && lastLine.trim() && PROMPT.test(lastLine)) {
      this.#told = true;
      return 'done';
    }
    return null;
  }
}
