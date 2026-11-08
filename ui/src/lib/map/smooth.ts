/**
 * Softens the sharp corners of the generated 110m country outlines (borders.ts).
 * Each corner is cut at most `r` units from its vertex (less on short edges) and joined with a
 * quadratic curve, so straight borders stay straight while jagged coasts lose their saw teeth.
 * Input uses the generator's format only: absolute `M x y`, relative `l dx dy`, `z`; every
 * sub-path there is a closed ring.
 */
type Pt = [number, number];

const num = (n: number) => String(Math.round(n * 10) / 10);
const at = (p: Pt) => `${num(p[0])} ${num(p[1])}`;
const dist = (a: Pt, b: Pt) => Math.hypot(b[0] - a[0], b[1] - a[1]);
/** Point on the edge from `a` towards `b`, `d` units from `a`. */
const toward = (a: Pt, b: Pt, d: number): Pt => {
  const l = dist(a, b) || 1;
  return [a[0] + ((b[0] - a[0]) * d) / l, a[1] + ((b[1] - a[1]) * d) / l];
};

function ring(raw: Pt[], r: number): string {
  // Repeated points have no direction to round.
  const same = (a: Pt | undefined, b: Pt | undefined) => a?.[0] === b?.[0] && a?.[1] === b?.[1];
  const p = raw.filter((q, i) => i === 0 || !same(q, raw[i - 1]));
  if (p.length > 1 && same(p[0], p[p.length - 1])) p.pop();
  const n = p.length;
  const pt = (i: number) => p[((i % n) + n) % n] as Pt;
  if (n < 3)
    return n
      ? `M${at(pt(0))}${p
          .slice(1)
          .map((q) => `L${at(q)}`)
          .join('')}Z`
      : '';
  // Entry and exit point of the rounded corner at vertex i.
  const corner = (i: number): [Pt, Pt] => {
    const prev = pt(i - 1);
    const next = pt(i + 1);
    const cur = pt(i);
    return [
      toward(cur, prev, Math.min(r, dist(cur, prev) / 2)),
      toward(cur, next, Math.min(r, dist(cur, next) / 2))
    ];
  };
  let d = `M${at(corner(0)[1])}`;
  for (let k = 1; k <= n; k++) {
    const i = k % n;
    const [enter, exit] = corner(i);
    d += `L${at(enter)}Q${at(pt(i))} ${at(exit)}`;
  }
  return d + 'Z';
}

/** Smooths one generated path string (any number of sub-paths). */
export function smoothPath(d: string, r = 6): string {
  let out = '';
  let pts: Pt[] = [];
  let x = 0;
  let y = 0;
  const flush = () => {
    if (pts.length) out += ring(pts, r);
    pts = [];
  };
  for (const [, cmd, args = ''] of d.matchAll(/([Mlz])([^Mlz]*)/g)) {
    if (cmd === 'z') {
      flush();
      continue;
    }
    const v = args.match(/-?\d+(?:\.\d+)?/g)?.map(Number) ?? [];
    if (cmd === 'M') {
      flush();
      [x, y] = [v[0] ?? 0, v[1] ?? 0];
      pts.push([x, y]);
      continue;
    }
    for (let i = 0; i + 1 < v.length; i += 2) {
      x += v[i] ?? 0;
      y += v[i + 1] ?? 0;
      pts.push([x, y]);
    }
  }
  flush();
  return out;
}
