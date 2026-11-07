/** Idle auto-lock helpers (`security.md`: idle 15 min, 0 = never). Pure, so they are testable. */

/** Settings key holding the idle timeout in minutes. */
export const AUTOLOCK_KEY = 'security.autolock_minutes';
/** Default idle timeout. */
export const AUTOLOCK_DEFAULT = 15;
/** Choices offered in Settings (0 = never). */
export const AUTOLOCK_CHOICES = [1, 5, 15, 30, 60, 0] as const;

/** Validates a stored value; anything unusual falls back to the default. */
export function parseAutolock(v: unknown): number {
  if (typeof v !== 'number' || !Number.isFinite(v) || v < 0 || v > 24 * 60) return AUTOLOCK_DEFAULT;
  return Math.round(v);
}

/** Events that count as user activity. */
const ACTIVITY = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'] as const;

/** The slice of `window` that `watchIdle` needs. */
export type IdleTarget = Pick<Window, 'addEventListener' | 'removeEventListener'>;

/**
 * Calls `onIdle` after `minutes` without activity. Returns a stop function.
 * `minutes <= 0` disables the timer. Resets are throttled to once per second.
 */
export function watchIdle(
  minutes: number,
  onIdle: () => void,
  target: IdleTarget = window
): () => void {
  if (minutes <= 0) return () => {};
  const ms = minutes * 60_000;
  let timer = setTimeout(onIdle, ms);
  let last = Date.now();
  const bump = () => {
    const now = Date.now();
    if (now - last < 1000) return;
    last = now;
    clearTimeout(timer);
    timer = setTimeout(onIdle, ms);
  };
  for (const e of ACTIVITY) target.addEventListener(e, bump, { passive: true });
  return () => {
    clearTimeout(timer);
    for (const e of ACTIVITY) target.removeEventListener(e, bump);
  };
}
