/**
 * Colours for container logs: the container's own ANSI colours when it prints them, otherwise a
 * light highlighting of timestamps, levels, HTTP methods / statuses, addresses and `key=`.
 * Output is plain segments rendered as text (never HTML).
 */

export interface Seg {
  text: string;
  /** Space-separated class names from a fixed set (`ts`, `err`, `a3`, `b`…), or ''. */
  cls: string;
}

// eslint-disable-next-line no-control-regex
const SGR = /\x1b\[([\d;]*)m/g;

/** Removes colour codes (for copying). */
export const stripAnsi = (s: string) => s.replace(SGR, '');

/** Nearest of the 8 base colours for an RGB value; `null` for greys (shown as plain text). */
function nearest(r: number, g: number, b: number): number | null {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  if (max - min < 40) return max < 90 ? 8 : null;
  const d = max - min;
  let h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h = (h * 60 + 360) % 360;
  if (h < 20 || h >= 330) return 1;
  if (h < 70) return 3;
  if (h < 160) return 2;
  if (h < 200) return 6;
  if (h < 260) return 4;
  return 5;
}

/** xterm 256-colour index → one of the 16 colour classes (or `null`). */
function from256(n: number): number | null {
  if (n < 16) return n;
  if (n >= 232) return n < 244 ? 8 : null;
  const v = [0, 95, 135, 175, 215, 255];
  const i = n - 16;
  return nearest(v[Math.floor(i / 36)]!, v[Math.floor(i / 6) % 6]!, v[i % 6]!);
}

/**
 * Splits a line with SGR codes into coloured segments (16 colours; 256 and RGB map to the
 * nearest of them). Text the program left in the default colour still gets the plain rules.
 */
function ansi(line: string): Seg[] {
  const out: Seg[] = [];
  let fg: number | null = null;
  let bold = false;
  let dim = false;
  let last = 0;
  const push = (text: string) => {
    if (!text) return;
    if (fg == null && !dim) {
      for (const seg of plain(text))
        out.push(bold ? { text: seg.text, cls: seg.cls ? `${seg.cls} b` : 'b' } : seg);
      return;
    }
    const cls = [fg != null ? `a${fg}` : '', bold ? 'b' : '', dim ? 'dim' : '']
      .filter(Boolean)
      .join(' ');
    out.push({ text, cls });
  };
  for (const mt of line.matchAll(SGR)) {
    push(line.slice(last, mt.index));
    last = mt.index + mt[0].length;
    const codes = (mt[1] || '0').split(';').map(Number);
    for (let i = 0; i < codes.length; i++) {
      const c = codes[i]!;
      if (c === 0) [fg, bold, dim] = [null, false, false];
      else if (c === 1) bold = true;
      else if (c === 2) dim = true;
      else if (c === 22) [bold, dim] = [false, false];
      else if (c >= 30 && c <= 37) fg = c - 30;
      else if (c >= 90 && c <= 97) fg = c - 90 + 8;
      else if (c === 39) fg = null;
      else if (c === 38 && codes[i + 1] === 5) {
        fg = from256(codes[i + 2] ?? 7);
        i += 2;
      } else if (c === 38 && codes[i + 1] === 2) {
        fg = nearest(codes[i + 2] ?? 0, codes[i + 3] ?? 0, codes[i + 4] ?? 0);
        i += 4;
      } else if (c === 48) i += codes[i + 1] === 5 ? 2 : 4; // backgrounds: not shown
    }
  }
  push(line.slice(last));
  return out;
}

const TS = /^\d{4}-\d\d-\d\dT[\d:.]+(?:Z|[+-]\d\d:?\d\d)?\s/;
/** Order matters: at one position the first group that matches wins. */
const TOKENS = new RegExp(
  [
    // Dates inside the message: nginx, Postgres, Redis, Go, Apache.
    String.raw`(?<dt>\[?\d{4}[-/]\d\d[-/]\d\d[ T]\d\d:\d\d:\d\d(?:[.,]\d+)?(?:Z|\s?UTC|\s?[+-]\d\d:?\d\d)?\]?|\[\d\d/\w{3}/\d{4}(?::\d\d){3} [+-]\d{4}\]|\b\d\d? \w{3} \d{4} \d\d:\d\d:\d\d(?:\.\d+)?)`,
    String.raw`(?<url>https?://[^\s"'<>]+[^\s"'<>.,;:)\]])`,
    String.raw`\b(?<err>ERROR|ERR|FATAL|CRIT(?:ICAL)?|PANIC|EMERG|ALERT|SEVERE|FAIL(?:ED)?|[Ee]rror|[Ff]atal|[Cc]ritical|[Pp]anic|crit|emerg|[Ee]xception|Traceback|[Ff]ailed)\b`,
    String.raw`\b(?<warn>WARN(?:ING)?|[Ww]arn(?:ing)?|DEPRECATED|[Dd]eprecated)\b`,
    String.raw`\b(?<info>INFO|[Ii]nfo|NOTICE|[Nn]otice|NOTE|Note|LOG(?=:)|System(?=\]))\b`,
    String.raw`\b(?<dbg>DEBUG|[Dd]ebug|TRACE|[Tt]race|VERBOSE)\b`,
    String.raw`\b(?<good>ready|started|listening|connected|healthy|succe(?:ss(?:ful(?:ly)?)?|eded)|OK|done)\b`,
    String.raw`\b(?<meth>GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\b`,
    String.raw`" (?<status>[1-5]\d\d) `,
    String.raw`\b(?<ip>(?:\d{1,3}\.){3}\d{1,3})(?::\d+)?\b`,
    String.raw`"(?<jkey>[A-Za-z_][\w.-]*)":`,
    String.raw`\b(?<key>[A-Za-z_][\w.-]*)=`,
    String.raw`\b(?<num>\d+(?:[.,]\d+)?\s?(?:ms|µs|us|ns|s|%|[KMGT]i?B|B))(?![\w])`
  ].join('|'),
  'g'
);
const CLS: Record<string, string> = {
  dt: 'dim',
  url: 'url',
  err: 'err b',
  warn: 'warn b',
  info: 'info b',
  dbg: 'dbg b',
  good: 'ok',
  meth: 'meth',
  ip: 'ip',
  num: 'num'
};

/** Light highlighting for text without colour codes. */
function plain(rest: string): Seg[] {
  const out: Seg[] = [];
  let last = 0;
  for (const mt of rest.matchAll(TOKENS)) {
    const g = mt.groups ?? {};
    const kind = Object.keys(g).find((k) => g[k] !== undefined);
    if (!kind) continue;
    if (mt.index > last) out.push({ text: rest.slice(last, mt.index), cls: '' });
    if (kind === 'status') {
      const code = g.status!;
      const tone =
        code[0] === '5' ? 'err' : code[0] === '4' ? 'warn' : code[0] === '2' ? 'ok' : 'info';
      out.push({ text: '" ', cls: '' }, { text: code, cls: `${tone} b` }, { text: ' ', cls: '' });
    } else if (kind === 'key') {
      out.push({ text: g.key!, cls: 'key' }, { text: '=', cls: 'dim' });
    } else if (kind === 'jkey') {
      out.push({ text: `"${g.jkey!}"`, cls: 'key' }, { text: ':', cls: 'dim' });
    } else {
      out.push({ text: mt[0], cls: CLS[kind] ?? '' });
    }
    last = mt.index + mt[0].length;
  }
  if (last < rest.length) out.push({ text: rest.slice(last), cls: '' });
  return out;
}

/** `2026-10-31T21:05:05.677Z` → `31 Oct 21:05:05` in the viewer's time zone, like journalctl. */
export function shortTime(iso: string, locale: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const month = d.toLocaleDateString(locale, { month: 'short' }).replace('.', '');
  const day = `${String(d.getDate()).padStart(2, '0')} ${month}`;
  const time = d.toLocaleTimeString(locale, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
  return `${day} ${time}`;
}

export interface LogLine {
  /** Docker's timestamp (full), if the line starts with one. */
  ts: string | null;
  segs: Seg[];
}

/** One log line: the leading Docker timestamp apart, the rest coloured. */
export function colorLine(line: string): LogLine {
  const m = TS.exec(line);
  const ts = m ? m[0].trim() : null;
  const rest = m ? line.slice(m[0].length) : line;
  return { ts, segs: rest.includes('\x1b[') ? ansi(rest) : plain(rest) };
}
