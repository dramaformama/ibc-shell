/** Parser for the palette's quick-connect input: `[user@]host[:port]`, `[::1]:22`. */
import type { QuickConnect } from '$lib/api';

const HOST_RE = /^[a-z0-9]([a-z0-9._-]*[a-z0-9])?$/i;
const IPV6_RE = /^[0-9a-f:]+$/i;

/** Returns the target, or `null` when the text is not a plausible connection string. */
export function parseQuickConnect(input: string): QuickConnect | null {
  const s = input.trim();
  if (!s || /\s/.test(s)) return null;

  let user: string | null = null;
  let rest = s;
  const at = s.lastIndexOf('@');
  if (at !== -1) {
    user = s.slice(0, at);
    rest = s.slice(at + 1);
    if (!user || /[@:]/.test(user)) return null;
  }

  let hostname = rest;
  let port: number | null = null;
  if (rest.startsWith('[')) {
    const end = rest.indexOf(']');
    if (end === -1) return null;
    hostname = rest.slice(1, end);
    const tail = rest.slice(end + 1);
    if (tail) {
      if (!tail.startsWith(':')) return null;
      port = parsePort(tail.slice(1));
      if (port === null) return null;
    }
    if (!hostname.includes(':') || !IPV6_RE.test(hostname)) return null;
  } else {
    const colon = rest.lastIndexOf(':');
    if (colon !== -1) {
      hostname = rest.slice(0, colon);
      port = parsePort(rest.slice(colon + 1));
      if (port === null) return null;
    }
    if (!HOST_RE.test(hostname)) return null;
    // Without `user@` a bare word is ambiguous: require a dot, a port or an IP.
    if (!user && port === null && !hostname.includes('.')) return null;
  }
  return { user, hostname, port };
}

function parsePort(text: string): number | null {
  if (!/^\d{1,5}$/.test(text)) return null;
  const n = Number(text);
  return n >= 1 && n <= 65535 ? n : null;
}
