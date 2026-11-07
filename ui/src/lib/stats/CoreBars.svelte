<script lang="ts">
  import type { CoreSplit } from '$lib/api';
  import { pct } from './format';

  /**
   * Per-core load as rows of segments, one row per core. With `split` each row is coloured by
   * kind of time (user, system, nice, I/O wait, steal), so a busy app and a stalled disk look
   * different at a glance; without it the row is coloured by load.
   */
  let {
    cores,
    split = [],
    columns = 1
  }: { cores: (number | null)[]; split?: CoreSplit[]; columns?: 1 | 2 } = $props();
  const SEG = 24;
  const KINDS = ['user', 'system', 'nice', 'iowait', 'steal'] as const;
  const lit = (p: number) => Math.round((Math.min(100, Math.max(0, p)) / 100) * SEG);
  const tone = (i: number, p: number) =>
    i >= SEG * 0.85 && p >= 85 ? 'hot' : i >= SEG * 0.6 && p >= 60 ? 'warm' : 'calm';

  /** Segment classes of one core: runs of each kind, rounded so the total stays ≤ 24. */
  function segments(c: number, p: number): string[] {
    const s = split[c];
    if (!s) {
      const n = lit(p);
      return Array.from({ length: SEG }, (_, i) => (i < n ? tone(i, p) : ''));
    }
    const out: string[] = [];
    let acc = 0;
    for (const k of KINDS) {
      acc += s[k] ?? 0;
      const upto = Math.min(SEG, lit(acc));
      while (out.length < upto) out.push(k);
    }
    while (out.length < SEG) out.push('');
    return out;
  }
</script>

<div class="bars" class:two={columns === 2}>
  {#each cores as p, c (c)}
    <div class="row">
      <div class="segs" aria-hidden="true">
        {#each segments(c, p ?? 0) as k, i (i)}<span class={k}></span>{/each}
      </div>
      <span class="v tnum">{pct(p)}<small>%</small></span>
    </div>
  {/each}
</div>

<style>
  .bars {
    display: grid;
    gap: 5px;
  }
  .bars.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    column-gap: 18px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .segs {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(24, 1fr);
    gap: 2px;
  }
  .segs span {
    height: 9px;
    border-radius: 3px;
    background: var(--surface-2);
    transition: background-color 400ms var(--ease);
  }
  .segs span.calm,
  .segs span.user {
    background: var(--c-cyan);
  }
  .segs span.warm {
    background: var(--c-yellow);
  }
  .segs span.hot,
  .segs span.system {
    background: var(--danger);
  }
  .segs span.nice {
    background: var(--c-green);
  }
  .segs span.iowait {
    background: var(--c-purple);
  }
  .segs span.steal {
    background: var(--c-orange);
  }
  .v {
    width: 36px;
    text-align: right;
    font-size: 12px;
    font-weight: 600;
  }
  small {
    color: var(--muted);
    font-weight: 500;
    font-size: 10px;
  }
</style>
