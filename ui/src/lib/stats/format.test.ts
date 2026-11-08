import { describe, expect, it } from 'vitest';
import { bytes, decimal, loadColor, pct, percent, rate, uptime } from './format';

describe('stats format', () => {
  it('formats sizes and rates', () => {
    expect(bytes(16 * 1024 ** 3)).toBe('16.0 GB');
    expect(bytes(452 * 1024 ** 2)).toBe('452 MB');
    expect(bytes(512)).toBe('512 B');
    expect(rate(4.01 * 1024 ** 2)).toEqual(['4.0', 'MB/s']);
  });
  it('distinguishes missing metrics from idle values', () => {
    expect(pct(null)).toBe('—');
    expect(percent(null)).toBe('—');
    expect(percent(0)).toBe('0%');
    expect(pct(0)).toBe('0');
    expect(decimal(null)).toBe('—');
    expect(decimal(0)).toBe('0.00');
    expect(loadColor(null)).toBe('var(--muted)');
  });
  it('formats uptime', () => {
    expect(uptime(6 * 86400 + 2 * 3600)).toBe('6 d 2 h');
    expect(uptime(3 * 3600 + 12 * 60)).toBe('3 h 12 m');
    expect(uptime(45 * 60)).toBe('45 m');
  });
});
