/** Pure snippet logic: variables, keystroke text, folder layout. No IO. */

export interface SnippetVar {
  name: string;
  /** Text after `:` in `{{name:default}}`, or ''. */
  fallback: string;
}

const VAR = /\{\{\s*([\p{L}\p{N}_-]{1,40})\s*(?::([^}]*))?\}\}/gu;

/** Distinct `{{name}}` / `{{name:default}}` placeholders, in order of appearance. */
export function parseVars(body: string): SnippetVar[] {
  const out: SnippetVar[] = [];
  for (const m of body.matchAll(VAR)) {
    const name = m[1]!;
    if (!out.some((v) => v.name === name)) out.push({ name, fallback: (m[2] ?? '').trim() });
  }
  return out;
}

/** Replaces placeholders; a missing value falls back to the default. */
export function fillVars(body: string, values: Record<string, string>): string {
  return body.replace(VAR, (_, name: string, fb?: string) => values[name] ?? (fb ?? '').trim());
}

/**
 * What to type into the shell: newlines become Enter. `execute` presses Enter after the last
 * line too; otherwise the last line stays on the prompt for editing.
 */
export function keystrokes(body: string, execute: boolean): string {
  const lines = body.replace(/\r\n?/g, '\n').replace(/\n+$/, '').split('\n');
  return lines.join('\r') + (execute ? '\r' : '');
}

export interface SnippetGroup {
  id: string;
  name: string;
}

/** Organization kept in settings (`snippets.layout`); snippet bodies live in the store. */
export interface Layout {
  groups: SnippetGroup[];
  /** Snippet id → group id (absent = "No folder"). */
  of: Record<string, string>;
  /** Manual order of snippet ids across all groups. */
  order: string[];
  /** Run counts, for "Frequent". */
  uses: Record<string, number>;
  collapsed: string[];
}

export const emptyLayout = (): Layout => ({
  groups: [],
  of: {},
  order: [],
  uses: {},
  collapsed: []
});

/** Accepts only well-formed parts of a stored layout. */
export function readLayout(v: unknown): Layout {
  const l = emptyLayout();
  if (!v || typeof v !== 'object') return l;
  const o = v as Partial<Record<keyof Layout, unknown>>;
  if (Array.isArray(o.groups))
    l.groups = o.groups.filter(
      (g): g is SnippetGroup =>
        !!g && typeof g.id === 'string' && typeof g.name === 'string' && !!g.name
    );
  const strMap = (x: unknown) =>
    x && typeof x === 'object'
      ? Object.fromEntries(Object.entries(x).filter(([, val]) => typeof val === 'string'))
      : {};
  l.of = strMap(o.of) as Record<string, string>;
  if (Array.isArray(o.order)) l.order = o.order.filter((x): x is string => typeof x === 'string');
  if (o.uses && typeof o.uses === 'object')
    l.uses = Object.fromEntries(
      Object.entries(o.uses).filter(([, n]) => typeof n === 'number' && n > 0)
    ) as Record<string, number>;
  if (Array.isArray(o.collapsed))
    l.collapsed = o.collapsed.filter((x): x is string => typeof x === 'string');
  return l;
}

export interface Named {
  id: string;
  name: string;
  body: string;
}

export interface Section<T> {
  group: SnippetGroup | null;
  items: T[];
}

/** Snippets sorted by manual order (unknown ids last, by name) and split into folders. */
export function sections<T extends Named>(items: T[], l: Layout, query = ''): Section<T>[] {
  const q = query.trim().toLocaleLowerCase();
  const rank = new Map(l.order.map((id, i) => [id, i]));
  const sorted = items
    .filter((s) => !q || `${s.name}\n${s.body}`.toLocaleLowerCase().includes(q))
    .sort(
      (a, b) =>
        (rank.get(a.id) ?? Infinity) - (rank.get(b.id) ?? Infinity) || a.name.localeCompare(b.name)
    );
  const known = new Set(l.groups.map((g) => g.id));
  const groupOf = (s: T) => (known.has(l.of[s.id] ?? '') ? l.of[s.id]! : null);
  const out: Section<T>[] = l.groups.map((g) => ({
    group: g,
    items: sorted.filter((s) => groupOf(s) === g.id)
  }));
  const loose = sorted.filter((s) => groupOf(s) === null);
  if (loose.length || !l.groups.length) out.push({ group: null, items: loose });
  return q ? out.filter((s) => s.items.length) : out;
}

/** Up to `n` snippets run at least twice, most used first. */
export function frequent<T extends Named>(items: T[], l: Layout, n = 4): T[] {
  return items
    .filter((s) => (l.uses[s.id] ?? 0) >= 2)
    .sort((a, b) => (l.uses[b.id] ?? 0) - (l.uses[a.id] ?? 0))
    .slice(0, n);
}

/**
 * Moves `id` into `group` (null = no folder), placed before `before` (null = end of the
 * group). Returns a new layout; `all` is the current id list for completing the order.
 */
export function moveSnippet(
  l: Layout,
  all: string[],
  id: string,
  group: string | null,
  before: string | null
): Layout {
  const order = [...l.order, ...all.filter((x) => !l.order.includes(x))].filter((x) => x !== id);
  const of = { ...l.of };
  if (group) of[id] = group;
  else delete of[id];
  let at = before ? order.indexOf(before) : -1;
  if (at === -1) {
    // End of the target group: after its last member, or at the very end.
    const members = order.filter((x) => (of[x] ?? null) === group);
    const last = members[members.length - 1];
    at = last ? order.indexOf(last) + 1 : order.length;
  }
  order.splice(at, 0, id);
  return { ...l, of, order };
}
