/**
 * Resolves once the window is visible (immediately if it already is). Background polling loops
 * await this so a minimized or hidden window sends no metric samples or port probes (ADR-009).
 */
export const hidden = (): boolean =>
  typeof document !== 'undefined' && document.visibilityState === 'hidden';

export function whenVisible(): Promise<void> {
  if (!hidden()) return Promise.resolve();
  return new Promise((resolve) => {
    const on = () => {
      if (document.visibilityState === 'hidden') return;
      document.removeEventListener('visibilitychange', on);
      resolve();
    };
    document.addEventListener('visibilitychange', on);
  });
}
