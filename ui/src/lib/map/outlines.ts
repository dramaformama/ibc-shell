import { OUTLINES } from './borders';
import { smoothPath } from './smooth';

/** Country outlines with softened corners, computed once on first use. */
export const SMOOTH_OUTLINES: Record<string, string> = Object.fromEntries(
  Object.entries(OUTLINES).map(([cc, d]) => [cc, smoothPath(d)])
);
