/**
 * Output highlighting (Termius-style): colours IPs, URLs, errors, warnings, success words and
 * sizes in plain server output. Only touches text printed with default colours on the normal
 * screen, so programs that colour their own output, and full-screen apps (vim, top), are left
 * exactly as sent. Escape sequences split across chunks are carried over.
 */

const RULES: [RegExp, number][] = [
  [/https?:\/\/[^\s"'<>]+[^\s"'<>.,;:)\]]/, 34],
  [/\b(?:\d{1,3}\.){3}\d{1,3}(?:\/\d{1,2})?(?::\d{1,5})?\b/, 36],
  [/\b(?:error|errors|failed|failure|fatal|denied|refused|panic|critical|exception)\b/i, 31],
  [/\b(?:warn|warning|warnings|deprecated)\b/i, 33],
  [/\b(?:ok|success|successful|succeeded|active|enabled|passed|done)\b/i, 32],
  [/\b\d+(?:[.,]\d+)?\s?(?:%|[KMGT]i?B|[KMGT]B?\/s)(?![\w])/, 35]
];
const ALL = new RegExp(RULES.map(([r]) => `(${r.source})`).join('|'), 'gi');
// eslint-disable-next-line no-control-regex -- matching terminal escape sequences is the point
const CSI = /^\x1b\[([0-9;?]*)([ -/]*)([@-~])/;

export class Highlighter {
  /** Default foreground in effect (we only colour text the program did not colour). */
  #plain = true;
  /** Alternate screen (full-screen app) active. */
  #alt = false;
  /** Incomplete escape sequence from the previous chunk. */
  #carry = '';

  feed(chunk: string): string {
    const s = this.#carry + chunk;
    this.#carry = '';
    let out = '';
    let i = 0;
    while (i < s.length) {
      const esc = s.indexOf('\x1b', i);
      const end = esc === -1 ? s.length : esc;
      if (end > i) out += this.#text(s.slice(i, end));
      if (esc === -1) break;
      const n = this.#escape(s, esc);
      if (n === 0) {
        this.#carry = s.slice(esc);
        break;
      }
      out += s.slice(esc, esc + n);
      i = esc + n;
    }
    return out;
  }

  /** Length of the escape sequence at `at`, 0 when it is incomplete. */
  #escape(s: string, at: number): number {
    if (at + 1 >= s.length) return 0;
    const kind = s[at + 1];
    if (kind === '[') {
      const m = CSI.exec(s.slice(at, at + 64));
      if (!m) return s.length - at < 64 ? 0 : 2;
      const [all, params = '', , final] = m;
      if (final === 'm' && !params.startsWith('?')) {
        this.#plain = /^[0;]*$/.test(params) || params === '39' || /(^|;)0;?39?$/.test(params);
      } else if ((final === 'h' || final === 'l') && /^\?(1049|1047|47)$/.test(params)) {
        this.#alt = final === 'h';
      }
      return all.length;
    }
    if (kind === ']' || kind === 'P' || kind === '_' || kind === '^') {
      for (let j = at + 2; j < s.length; j++) {
        if (s[j] === '\x07') return j - at + 1;
        if (s[j] === '\x1b' && s[j + 1] === '\\') return j - at + 2;
        if (s[j] === '\x1b' && j + 1 >= s.length) return 0;
      }
      return 0;
    }
    return 2;
  }

  #text(t: string): string {
    if (!this.#plain || this.#alt) return t;
    return t.replace(ALL, (match, ...groups: unknown[]) => {
      const idx = groups.findIndex((g, k) => k < RULES.length && g !== undefined);
      const color = RULES[idx]?.[1];
      return color ? `\x1b[${color}m${match}\x1b[39m` : match;
    });
  }
}
