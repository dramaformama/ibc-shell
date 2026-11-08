import { describe, expect, it } from 'vitest';
import { OUTLINES } from './borders';
import { smoothPath } from './smooth';

describe('smoothPath', () => {
  it('rounds a square: keeps the straight middles, curves the corners', () => {
    const d = smoothPath('M0 0l100 0l0 100l-100 0z', 6);
    expect(d.startsWith('M6 0')).toBe(true);
    expect(d.match(/Q/g)).toHaveLength(4);
    expect(d.endsWith('Z')).toBe(true);
  });
  it('splits sub-paths and tolerates degenerate rings', () => {
    const d = smoothPath('M0 0l10 0l0 10l-10 0zM50 50l2 0z', 6);
    expect(d.match(/M/g)).toHaveLength(2);
  });
  it('turns every real outline into finite numbers only', () => {
    for (const d of Object.values(OUTLINES)) expect(smoothPath(d)).not.toMatch(/NaN|Infinity/);
  });
});
