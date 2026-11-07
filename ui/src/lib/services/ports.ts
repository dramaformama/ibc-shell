/** Compact view of `docker ps` port strings and certificate expiry helpers. */

export interface PortChip {
  /** `443` or `8080→80`. */
  text: string;
  proto: string;
  /** Bound to loopback only (not reachable from outside). */
  local: boolean;
  /** Exposed by the image but not published on the host. */
  internal: boolean;
}

/** `0.0.0.0:443->443/tcp, :::443->443/tcp, 127.0.0.1:5432->5432/tcp` → unique chips. */
export function portChips(ports: string): PortChip[] {
  const out = new Map<string, PortChip>();
  for (const raw of ports.split(',')) {
    const p = raw.trim();
    if (!p) continue;
    const pub = /^(.*):(\d+(?:-\d+)?)->(\d+(?:-\d+)?)\/(\w+)$/.exec(p);
    if (pub) {
      const [, host = '', from = '', to = '', proto = ''] = pub;
      const local = host.startsWith('127.') || host === '[::1]' || host === '::1';
      const text = from === to ? from : `${from}→${to}`;
      const key = `${text}/${proto}`;
      const prev = out.get(key);
      // The same port on 0.0.0.0 and :: is one public port.
      out.set(key, { text, proto, local: prev ? prev.local && local : local, internal: false });
      continue;
    }
    const exp = /^(\d+(?:-\d+)?)\/(\w+)$/.exec(p);
    if (exp) {
      const [, text = '', proto = ''] = exp;
      const key = `${text}/${proto}`;
      if (!out.has(key)) out.set(key, { text, proto, local: false, internal: true });
    }
  }
  return [...out.values()];
}

const DAY = 86_400_000;

/** Whole days left until `at` (negative when past). */
export const daysLeft = (at: number, now = Date.now()) => Math.floor((at - now) / DAY);

/** `bad` ≤ 7 days, `warn` ≤ 21, else `ok`. */
export function certTone(days: number): 'bad' | 'warn' | 'ok' {
  return days <= 7 ? 'bad' : days <= 21 ? 'warn' : 'ok';
}
