<script lang="ts">
  import type { CoreSplit } from '$lib/api';
  import { pct } from './format';

  /**
   * Per-core load, one bar per core. With `split` the bar is divided by kind of time (user,
   * system, nice, I/O wait, steal), so a busy app and a stalled disk look different at a
   * glance; without it the bar is coloured by load.
   */
  let {
    cores,
    split = [],
    columns = 1
  }: { cores: (number | null)[]; split?: CoreSplit[]; columns?: 1 | 2 } = $props();
  const KINDS = ['user', 'system', 'nice', 'iowait', 'steal'] as const;
  const clamp = (v: number) => Math.min(100, Math.max(0, v));
  const tone = (p: number) => (p >= 85 ? 'hot' : p >= 60 ? 'warm' : 'calm');

  /** Widths of the parts of one core's bar, summing to at most 100. */
  function parts(c: number, p: number): { cls: string; w: number }[] {
    const s = split[c];
    if (!s) return [{ cls: tone(p), w: clamp(p) }];
    let left = 100;
    return KINDS.map((k) => {
      const w = Math.min(left, clamp(s[k] ?? 0));
      left -= w;
      return { cls: k, w };
    });
  }
</script>

<div class="bars" class:two={columns === 2}>
  {#each cores as p, c (c)}
    <div class="row">
      <span class="n tnum">{c + 1}</span>
      <div class="bar" aria-hidden="true">
        {#each parts(c, p ?? 0) as part (part.cls)}<span class={part.cls} style:width="{part.w}%"
          ></span>{/each}
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
  .n {
    width: 14px;
    color: var(--muted);
    font-size: 10px;
    text-align: right;
  }
  .bar {
    flex: 1;
    display: flex;
    height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--fill-2);
  }
  .bar span {
    height: 100%;
  }
  .bar .calm,
  .bar .user {
    background: var(--c-cyan);
  }
  .bar .warm {
    background: var(--load-warm);
  }
  .bar .hot {
    background: var(--load-hot);
  }
  .bar .system {
    background: var(--c-orange);
  }
  .bar .nice {
    background: var(--c-yellow);
  }
  .bar .iowait {
    background: var(--c-purple);
  }
  .bar .steal {
    background: var(--c-pink);
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
