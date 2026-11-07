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

/** Splits a line with SGR codes into coloured segments (16 colours, bold, dim). */
function ansi(line: string): Seg[] {
  const out: Seg[] = [];
  let fg: number | null = null;
  let bold = false;
  let dim = false;
  let last = 0;
  const push = (text: string) => {
    if (!text) return;
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
      else if (c === 38 || c === 48) i += codes[i + 1] === 5 ? 2 : 4; // 256 / RGB: skipped
    }
  }
  push(line.slice(last));
  return out;
}

const TS = /^\d{4}-\d\d-\d\dT[\d:.]+(?:Z|[+-]\d\d:?\d\d)?\s/;
const TOKENS =
  /\b(?<err>ERROR|ERR|FATAL|CRIT(?:ICAL)?|PANIC|EMERG|error|fatal|panic)\b|\b(?<warn>WARN(?:ING)?|warn(?:ing)?)\b|\b(?<info>INFO|NOTICE|info|notice)\b|\b(?<dbg>DEBUG|TRACE|debug|trace)\b|\b(?<meth>GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\b|" (?<status>[1-5]\d\d) |\b(?<ip>(?:\d{1,3}\.){3}\d{1,3})(?::\d+)?\b|\b(?<key>[A-Za-z_][\w.-]*)=/g;

/** Light highlighting for lines without colour codes. */
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
    } else {
      out.push({ text: mt[0], cls: kind === 'ip' || kind === 'meth' ? kind : `${kind} b` });
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
