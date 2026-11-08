/**
 * The configuration of a server as it is sold: cores, memory, disk. Sizes are binary (what
 * `lsblk` and `free` report) but labelled GB / TB, which matches how hosting plans are named.
 */
import type { HostStats } from '$lib/api';

const GIB = 1024 ** 3;

/** `{ n, tb }`: a size in GB, or in TB from 1024 GB up. */
export function size(bytes: number): { n: string; tb: boolean } {
  const g = bytes / GIB;
  if (g >= 1024) return { n: (g / 1024).toFixed(1).replace(/\.0$/, ''), tb: true };
  // Memory shows a little less than its plan (the kernel keeps some): 15.6 → 16.
  if (g >= 2) return { n: String(Math.round(g)), tb: false };
  return { n: (Math.round(g * 2) / 2).toString(), tb: false };
}

/** The disk to describe: the physical device when known, else the root file system. */
export const diskBytes = (d: Pick<HostStats, 'disk_dev_bytes' | 'disk_total'>): number =>
  d.disk_dev_bytes || d.disk_total;

export type SpecInput = Pick<
  HostStats,
  'cores' | 'mem_total' | 'disk_dev_bytes' | 'disk_total' | 'disk_kind'
> & {
  /** Missing in samples remembered by older versions. */
  cpu_model?: string | null;
};

/** CPU model with the marketing noise removed: `Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz`. */
export function cpuShort(model: string | null | undefined): string | null {
  if (!model) return null;
  const s = model
    .replace(/\((R|TM|tm)\)/g, '')
    .replace(/\b(CPU|Processor)\b/g, '')
    .replace(/\s*@\s*[\d.]+\s*GHz/i, '')
    .replace(/\s+/g, ' ')
    .trim();
  return s || model;
}
