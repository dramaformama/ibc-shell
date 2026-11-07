/** Pure helpers: paste safety, title / URL sanitizing, byte encoding. */

/** Remove control characters that could break out of bracketed paste (ESC, C0 except \t \n \r, DEL). */
export function sanitizePaste(text: string): string {
  // eslint-disable-next-line no-control-regex
  return text.replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, '');
}

/** Number of lines a paste would submit; a single trailing newline does not count as a second line. */
export function pasteLineCount(text: string): number {
  const t = text.replace(/\r\n?/g, '\n').replace(/\n$/, '');
  return t === '' ? 0 : t.split('\n').length;
}

/** Multi-line pastes need confirmation: they can execute commands the moment they land. */
export const needsPasteConfirm = (text: string): boolean => pasteLineCount(text) > 1;

/** Window titles come from the remote side: strip controls, cap length. */
export function sanitizeTitle(raw: string): string {
  // eslint-disable-next-line no-control-regex
  return raw.replace(/[\x00-\x1f\x7f-\x9f‪-‮⁦-⁩]/g, '').slice(0, 80);
}

/** Only http(s) links may be opened. Returns the normalized URL or null. */
export function safeLink(raw: string): string | null {
  try {
    const u = new URL(raw);
    return u.protocol === 'http:' || u.protocol === 'https:' ? u.href : null;
  } catch {
    return null;
  }
}

const enc = new TextEncoder();

/** xterm `onData` string → UTF-8 bytes. */
export const utf8 = (s: string): Uint8Array => enc.encode(s);

/** xterm `onBinary` string (one char = one byte, latin1) → bytes. */
export function binaryBytes(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

/** Concatenate chunks into one buffer. */
export function concat(chunks: Uint8Array[]): Uint8Array {
  if (chunks.length === 1) return chunks[0]!;
  const out = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0));
  let o = 0;
  for (const c of chunks) {
    out.set(c, o);
    o += c.length;
  }
  return out;
}
