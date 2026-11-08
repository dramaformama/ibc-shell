/** Localized wording of a server's configuration (sizes and counts from `spec.ts`). */
import { getLocale } from '$lib/paraglide/runtime.js';
import * as m from '$lib/paraglide/messages.js';
import { diskBytes, size, type SpecInput } from './spec';

/** `8 cores` / `8 ядер`, with the language's plural form. */
export function coresText(n: number): string {
  const rule = new Intl.PluralRules(getLocale()).select(n);
  const f = {
    one: m.spec_core_one,
    few: m.spec_core_few,
    many: m.spec_core_many,
    other: m.spec_core_other,
    zero: m.spec_core_other,
    two: m.spec_core_other
  }[rule];
  return f({ n });
}

/** `16 GB`, `2 TB`. */
export function sizeText(bytes: number): string {
  const s = size(bytes);
  return s.tb ? m.spec_tb({ n: s.n }) : m.spec_gb({ n: s.n });
}

/** `250 GB NVMe`, or just the size when the kind is unknown; `null` without a disk. */
export function diskText(d: SpecInput): string | null {
  const b = diskBytes(d);
  if (!b) return null;
  const sz = sizeText(b);
  return d.disk_kind ? m.spec_disk_kind({ size: sz, kind: d.disk_kind }) : sz;
}

/** `8 cores · 16 GB · 250 GB NVMe`. */
export function specText(d: SpecInput): string {
  return [d.cores ? coresText(d.cores) : '', d.mem_total ? sizeText(d.mem_total) : '', diskText(d)]
    .filter(Boolean)
    .join(' · ');
}
