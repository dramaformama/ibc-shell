/** Server ordering shared by the sidebar tree and the overview: manual, name, last login, country. */
import { flagName, type FlagLocale } from '$lib/flags/flags';

export type HostSort = 'manual' | 'name' | 'last' | 'country';
export const HOST_SORTS: readonly HostSort[] = ['manual', 'name', 'last', 'country'];

/** What the comparators look at; both trees rows and overview servers map onto it. */
export interface SortFields {
  label: string;
  flag: string | null;
  /** Unix millis of the last successful connect. */
  last: number | null;
}

const collators = new Map<string, Intl.Collator>();
function collator(locale: string): Intl.Collator {
  let c = collators.get(locale);
  if (!c) {
    c = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
    collators.set(locale, c);
  }
  return c;
}

/**
 * Comparator for `sort`, or null for the manual order. Ties fall back to the name, and items
 * without a value (never connected, no flag) always go last.
 */
export function hostComparator(
  sort: HostSort,
  locale: FlagLocale
): ((a: SortFields, b: SortFields) => number) | null {
  if (sort === 'manual') return null;
  const byName = (a: SortFields, b: SortFields) => collator(locale).compare(a.label, b.label);
  if (sort === 'name') return byName;
  if (sort === 'last')
    return (a, b) =>
      a.last == null && b.last == null
        ? byName(a, b)
        : a.last == null
          ? 1
          : b.last == null
            ? -1
            : b.last - a.last || byName(a, b);
  return (a, b) =>
    !a.flag && !b.flag
      ? byName(a, b)
      : !a.flag
        ? 1
        : !b.flag
          ? -1
          : collator(locale).compare(flagName(a.flag, locale), flagName(b.flag, locale)) ||
            byName(a, b);
}

/** Reads a stored sort, ignoring junk and unavailable storage. */
export function loadSort(key: string): HostSort {
  try {
    const v = localStorage.getItem(key);
    return HOST_SORTS.includes(v as HostSort) ? (v as HostSort) : 'manual';
  } catch {
    return 'manual';
  }
}
export function saveSort(key: string, sort: HostSort): void {
  try {
    localStorage.setItem(key, sort);
  } catch {
    /* storage unavailable */
  }
}
