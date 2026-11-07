// Flag registry: codes, aliases, country names (Intl.DisplayNames, ru/en) and search.
// Contract: docs/contracts/ui.md §FlagPicker, ADR-005.
import { FLAG_CODES } from './codes';

export type FlagLocale = 'ru' | 'en';

/** Alias (lowercase, no surrounding spaces) → canonical flag code. */
export const ALIASES: Readonly<Record<string, string>> = {
  uk: 'gb',
  england: 'gb-eng',
  scotland: 'gb-sct',
  wales: 'gb-wls',
  usa: 'us',
  holland: 'nl',
  netherlands: 'nl',
  рф: 'ru',
  россия: 'ru',
  сша: 'us',
  германия: 'de',
  англия: 'gb-eng',
  великобритания: 'gb',
  голландия: 'nl',
  нидерланды: 'nl',
  украина: 'ua'
};

/** Names Intl.DisplayNames cannot provide (subdivisions and circle-flags placeholders). */
const EXTRA_NAMES: Readonly<Record<string, Readonly<Record<FlagLocale, string>>>> = {
  'gb-eng': { ru: 'Англия', en: 'England' },
  'gb-sct': { ru: 'Шотландия', en: 'Scotland' },
  'gb-wls': { ru: 'Уэльс', en: 'Wales' },
  'gb-nir': { ru: 'Северная Ирландия', en: 'Northern Ireland' },
  xx: { ru: 'Неизвестно', en: 'Unknown' }
};

const CODE_SET: ReadonlySet<string> = new Set(FLAG_CODES);
const displayNames = new Map<FlagLocale, Intl.DisplayNames>();
const nameCache = new Map<string, string>();

/** Folds case and ё→е so that "Ёлка" and "елка" compare equal. */
function fold(text: string): string {
  return text.trim().toLowerCase().replace(/ё/g, 'е');
}

/**
 * Resolves user input (code, alias, any case) to a known flag code.
 * Returns null when the input names no known flag.
 */
export function normalizeCode(input: string | null | undefined): string | null {
  if (!input) return null;
  const key = fold(input);
  if (key === '') return null;
  const direct = ALIASES[key] ?? key;
  return CODE_SET.has(direct) ? direct : null;
}

/** Whether `code` is one of the bundled flag codes (case-insensitive). */
export function isKnownFlag(code: string): boolean {
  return CODE_SET.has(fold(code));
}

/**
 * Human-readable country name in the given locale. Unknown codes fall back to the
 * upper-cased code, so the result is never empty.
 */
export function flagName(code: string, locale: FlagLocale = 'en'): string {
  const normalized = normalizeCode(code) ?? fold(code);
  const cacheKey = `${locale}:${normalized}`;
  const cached = nameCache.get(cacheKey);
  if (cached !== undefined) return cached;

  const name = EXTRA_NAMES[normalized]?.[locale] ?? regionName(normalized, locale);
  nameCache.set(cacheKey, name);
  return name;
}

function regionName(code: string, locale: FlagLocale): string {
  // Intl only knows region subtags; subdivisions like gb-eng are handled by EXTRA_NAMES.
  const upper = code.toUpperCase();
  if (!/^[A-Z]{2}$/.test(upper)) return upper;
  let names = displayNames.get(locale);
  if (!names) {
    names = new Intl.DisplayNames([locale], { type: 'region' });
    displayNames.set(locale, names);
  }
  return names.of(upper) ?? upper;
}

/** Scores how well `query` matches a code: lower is better, null means no match. */
function matchScore(
  query: string,
  code: string,
  aliasTargets: ReadonlyMap<string, string>
): number | null {
  let best: number | null = null;
  const consider = (score: number) => {
    if (best === null || score < best) best = score;
  };

  if (code === query) consider(0);
  else if (code.startsWith(query)) consider(1);

  for (const [alias, target] of aliasTargets) {
    if (target !== code) continue;
    if (alias === query) consider(0);
    else if (alias.startsWith(query)) consider(1);
  }

  for (const locale of ['ru', 'en'] as const) {
    const name = fold(flagName(code, locale));
    if (name.startsWith(query)) consider(2);
    else if (name.includes(query)) consider(3);
  }
  return best;
}

/**
 * Searches flags by code, alias, English or Russian name (both languages are always
 * searched). An empty query returns every code. Among equal matches, `recent` codes
 * come first in their recent order, then the rest in registry order.
 */
export function searchFlags(query: string, recent: readonly string[] = []): string[] {
  const q = fold(query);
  const recentRank = new Map<string, number>();
  recent.forEach((code, index) => {
    const normalized = normalizeCode(code);
    if (normalized && !recentRank.has(normalized)) recentRank.set(normalized, index);
  });
  const rankOf = (code: string) => recentRank.get(code) ?? Number.POSITIVE_INFINITY;

  if (q === '') {
    return [...FLAG_CODES].sort((a, b) => rankOf(a) - rankOf(b));
  }

  const aliasTargets = new Map(Object.entries(ALIASES));
  const scored: { code: string; score: number }[] = [];
  for (const code of FLAG_CODES) {
    const score = matchScore(q, code, aliasTargets);
    if (score !== null) scored.push({ code, score });
  }
  scored.sort(
    (a, b) => a.score - b.score || rankOf(a.code) - rankOf(b.code) || (a.code < b.code ? -1 : 1)
  );
  return scored.map((entry) => entry.code);
}
