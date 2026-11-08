import { describe, expect, it } from 'vitest';
import { TERM_THEMES } from '$lib/util/defaults-form';
import { TERMINAL_THEMES } from './themes';

const lum = (hex: string) => {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const [r, g, b] = c.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)) as [
    number,
    number,
    number
  ];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const ratio = (a: string, b: string) => {
  const [x, y] = [lum(a), lum(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
};

describe('terminal themes', () => {
  it('offers every theme it ships', () => {
    expect(TERM_THEMES.map((t) => t.id).sort()).toEqual(Object.keys(TERMINAL_THEMES).sort());
  });

  it('keeps white readable on the default light theme', () => {
    const t = TERMINAL_THEMES['ibc-light']!;
    expect(ratio(t.white!, t.background!)).toBeGreaterThanOrEqual(4.5);
    expect(ratio(t.brightWhite!, t.background!)).toBeGreaterThanOrEqual(4.5);
  });
});
