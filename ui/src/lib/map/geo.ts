/** Map geometry in dot-grid units: projection, view fitting and pin de-overlap. Pure. */
import { CENTROIDS, GRID } from './world';

export interface View {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const WORLD: View = { x: 0, y: 0, w: GRID.cols, h: GRID.rows };

/** Country label point → grid [col, row] (dot centres sit on integers). */
export function project(cc: string): [number, number] | null {
  const c = CENTROIDS[cc.toLowerCase()];
  if (!c) return null;
  return [(c[0] + 180) / GRID.step - 0.5, (GRID.lat0 - c[1]) / GRID.step - 0.5];
}

/**
 * The smallest view with the world's aspect ratio that shows every point with some margin,
 * never narrower than `minW` columns and never outside the world.
 */
export function fitView(points: [number, number][], minW = 48, pad = 9): View {
  if (!points.length) return WORLD;
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const ratio = GRID.rows / GRID.cols;
  const bw = Math.max(...xs) - Math.min(...xs) + pad * 2;
  const bh = Math.max(...ys) - Math.min(...ys) + pad * 2;
  const w = Math.min(GRID.cols, Math.max(minW, bw, bh / ratio));
  const h = w * ratio;
  const cx = (Math.max(...xs) + Math.min(...xs)) / 2;
  const cy = (Math.max(...ys) + Math.min(...ys)) / 2;
  return {
    x: Math.max(0, Math.min(GRID.cols - w, cx - w / 2)),
    y: Math.max(0, Math.min(GRID.rows - h, cy - h / 2)),
    w,
    h
  };
}

/**
 * Pushes overlapping pins apart (pixel space) along a small spiral, keeping each as close to
 * its true position as possible. Input order decides priority.
 */
export function spread(points: [number, number][], minDist: number): [number, number][] {
  const out: [number, number][] = [];
  for (const [x, y] of points) {
    let px = x;
    let py = y;
    for (let i = 0; i < 60; i++) {
      if (out.every(([ox, oy]) => Math.hypot(ox - px, oy - py) >= minDist)) break;
      const a = i * 2.4;
      const r = minDist * (0.6 + i * 0.12);
      px = x + Math.cos(a) * r;
      py = y + Math.sin(a) * r;
    }
    out.push([px, py]);
  }
  return out;
}
