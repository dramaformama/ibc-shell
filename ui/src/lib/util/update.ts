/** Update check against GitHub Releases. No identifiers are sent; the app only reads a public JSON. */

/** Settings key: whether the check runs (default on). */
export const UPDATE_CHECK_KEY = 'app.update_check';
/** Public releases endpoint (the only host the CSP allows besides the app itself). */
export const RELEASES_API = 'https://api.github.com/repos/dramaformama/ibc-shell/releases/latest';
/** Only links under this prefix are ever opened from an update notice. */
export const RELEASES_PAGE = 'https://github.com/dramaformama/ibc-shell/releases';

/** Parses `v1.2.3` / `1.2.3-beta` into numbers; null when it is not a plain version. */
export function parseVersion(v: string): [number, number, number] | null {
  const m = /^v?(\d{1,6})\.(\d{1,6})\.(\d{1,6})(?:[-+][0-9A-Za-z.-]*)?$/.exec(v.trim());
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** True when `latest` is a strictly newer version than `current`. */
export function isNewer(current: string, latest: string): boolean {
  const a = parseVersion(current);
  const b = parseVersion(latest);
  if (!a || !b) return false;
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return b[i]! > a[i]!;
  return false;
}

/** A newer release found on GitHub. */
export interface UpdateInfo {
  version: string;
  url: string;
}

/** Returns the newer release, or null (up to date, offline, or an unexpected reply). */
export async function checkForUpdate(
  current: string,
  fetcher: typeof fetch = fetch
): Promise<UpdateInfo | null> {
  try {
    const res = await fetcher(RELEASES_API, {
      headers: { Accept: 'application/vnd.github+json' },
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
      signal: AbortSignal.timeout(8000)
    });
    if (!res.ok) return null;
    const data: unknown = await res.json();
    if (typeof data !== 'object' || data === null) return null;
    const { tag_name: tag, html_url: url, draft, prerelease } = data as Record<string, unknown>;
    if (draft === true || prerelease === true) return null;
    if (typeof tag !== 'string' || !isNewer(current, tag)) return null;
    const link =
      typeof url === 'string' && url.startsWith(RELEASES_PAGE + '/') ? url : RELEASES_PAGE;
    return { version: tag.replace(/^v/, ''), url: link };
  } catch {
    return null;
  }
}
