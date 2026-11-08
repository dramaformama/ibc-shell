import { describe, expect, it } from 'vitest';
import { cpuShort, diskBytes, size } from './spec';

const GIB = 1024 ** 3;

describe('server configuration', () => {
  it('rounds memory to the plan size', () => {
    expect(size(15.6 * GIB)).toEqual({ n: '16', tb: false });
    expect(size(3.8 * GIB)).toEqual({ n: '4', tb: false });
    expect(size(0.97 * GIB)).toEqual({ n: '1', tb: false });
    expect(size(1.46 * GIB)).toEqual({ n: '1.5', tb: false });
  });

  it('switches to terabytes', () => {
    expect(size(250 * GIB)).toEqual({ n: '250', tb: false });
    expect(size(2048 * GIB)).toEqual({ n: '2', tb: true });
    expect(size(1536 * GIB)).toEqual({ n: '1.5', tb: true });
  });

  it('prefers the physical disk to the file system', () => {
    expect(diskBytes({ disk_dev_bytes: 80, disk_total: 70 })).toBe(80);
    expect(diskBytes({ disk_dev_bytes: 0, disk_total: 70 })).toBe(70);
  });

  it('shortens CPU names', () => {
    expect(cpuShort('Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz')).toBe('Intel Xeon E5-2680 v4');
    expect(cpuShort('AMD EPYC 7543 32-Core Processor')).toBe('AMD EPYC 7543 32-Core');
    expect(cpuShort(null)).toBeNull();
    expect(cpuShort('CPU')).toBe('CPU');
  });
});
