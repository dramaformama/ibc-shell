import { describe, expect, it } from 'vitest';
import { fitView, project, spread, WORLD } from './geo';
import { GRID } from './world';

describe('map geometry', () => {
  it('projects known countries inside the grid and rejects unknown codes', () => {
    const de = project('DE')!;
    expect(de[0]).toBeGreaterThan(GRID.cols / 2);
    expect(de[1]).toBeGreaterThan(0);
    expect(de[1]).toBeLessThan(GRID.rows / 2);
    expect(project('zz')).toBeNull();
  });

  it('zooms to a European cluster but keeps the world when servers span it', () => {
    const eu = fitView([project('de')!, project('nl')!, project('fi')!]);
    expect(eu.w).toBeLessThan(GRID.cols / 2);
    expect(eu.h / eu.w).toBeCloseTo(GRID.rows / GRID.cols);
    const all = fitView([project('us')!, project('au')!, project('fi')!]);
    expect(all.w).toBeGreaterThan(GRID.cols * 0.8);
    expect(all.x).toBeGreaterThanOrEqual(0);
    expect(all.x + all.w).toBeLessThanOrEqual(GRID.cols + 1e-9);
    expect(fitView([])).toEqual(WORLD);
  });

  it('separates overlapping pins and leaves distant ones alone', () => {
    const [a, b, c] = spread(
      [
        [100, 100],
        [104, 102],
        [400, 300]
      ],
      30
    );
    expect(a).toEqual([100, 100]);
    expect(Math.hypot(a![0] - b![0], a![1] - b![1])).toBeGreaterThanOrEqual(30);
    expect(c).toEqual([400, 300]);
  });
});
