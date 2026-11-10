/**
 * Where a terminal's shell currently is. Two signals, best first:
 * - OSC 7 `file://host/path` (zsh/fish setups, `vte.sh`, many prompts);
 * - the window title `user@host: dir` that Debian / Ubuntu's default bashrc sets.
 * Both are server-provided text: only a plain absolute or `~` path is accepted.
 */

/** Longest path accepted from the server. */
const MAX_PATH = 4096;

function clean(path: string): string | null {
  if (!path || path.length > MAX_PATH) return null;
  // eslint-disable-next-line no-control-regex -- rejecting control characters is the point
  if (/[\u0000-\u001f\u007f]/.test(path)) return null;
  if (!path.startsWith('/') && path !== '~' && !path.startsWith('~/')) return null;
  return path;
}

/** OSC 7 payload → absolute path, or null when it is not a `file:` URL. */
export function parseOsc7(data: string): string | null {
  const m = /^file:\/\/[^/]*(\/.*)$/.exec(data.trim());
  if (!m) return null;
  try {
    return clean(decodeURIComponent(m[1]!));
  } catch {
    return null;
  }
}

/** Title `user@host: ~/dir` (also `user@host:~/dir`) → `~/dir`; null for any other title. */
export function cwdFromTitle(title: string): string | null {
  const m = /^[\w.-]+@[\w.-]+:\s*(.+?)\s*$/.exec(title);
  return m ? clean(m[1]!) : null;
}

/** Resolves `~` against the remote home folder. */
export function expandHome(path: string, home: string): string {
  if (path === '~') return home;
  if (path.startsWith('~/')) return `${home.replace(/\/+$/, '')}/${path.slice(2)}`;
  return path;
}

/** Quotes local paths for a POSIX shell (dropping files onto the local terminal). */
export function shellQuote(path: string): string {
  return /^[\w@%+=:,./-]+$/.test(path) ? path : `'${path.replaceAll("'", `'\\''`)}'`;
}
