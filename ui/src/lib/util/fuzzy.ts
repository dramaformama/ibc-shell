/**
 * Tiny fuzzy scorer for the command palette (no dependencies).
 * Subsequence match; rewards consecutive runs, word starts and early hits.
 */

/** Returns a score > 0 when `query` matches `text` as a subsequence, otherwise 0. */
export function fuzzyScore(query: string, text: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;
  const t = text.toLowerCase();
  const exact = t.indexOf(q);
  if (exact !== -1) return 1000 - exact * 2 + (exact === 0 ? 200 : 0) - (t.length - q.length);

  let score = 0;
  let ti = 0;
  let run = 0;
  for (const ch of q) {
    const at = t.indexOf(ch, ti);
    if (at === -1) return 0;
    const wordStart = at === 0 || /[\s\-_./@:]/.test(t[at - 1] ?? '');
    run = at === ti ? run + 1 : 0;
    score += 10 + run * 8 + (wordStart ? 15 : 0) - Math.min(at - ti, 8);
    ti = at + 1;
  }
  return Math.max(score - t.length * 0.1, 1);
}

/** Best score across several fields (label, hostname, …). */
export function bestScore(query: string, ...fields: (string | null | undefined)[]): number {
  let best = 0;
  for (const f of fields) if (f) best = Math.max(best, fuzzyScore(query, f));
  return best;
}

/** Filters and sorts `items` by score, best first; ties keep input order. */
export function rank<T>(query: string, items: readonly T[], fields: (item: T) => string[]): T[] {
  const scored: { item: T; score: number; i: number }[] = [];
  items.forEach((item, i) => {
    const score = bestScore(query, ...fields(item));
    if (score > 0) scored.push({ item, score, i });
  });
  scored.sort((a, b) => b.score - a.score || a.i - b.i);
  return scored.map((s) => s.item);
}
