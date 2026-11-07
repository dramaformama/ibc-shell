<script lang="ts">
  import { pct } from './format';

  /** Per-core load as rows of segments (SwiftServer-style), one row per core. */
  let { cores, columns = 1 }: { cores: (number | null)[]; columns?: 1 | 2 } = $props();
  const SEG = 24;
  const lit = (p: number) => Math.round((Math.min(100, Math.max(0, p)) / 100) * SEG);
  const tone = (i: number, p: number) =>
    i >= SEG * 0.85 && p >= 85 ? 'hot' : i >= SEG * 0.6 && p >= 60 ? 'warm' : 'calm';
</script>

<div class="bars" class:two={columns === 2}>
  {#each cores as p, c (c)}
    {@const n = lit(p ?? 0)}
    <div class="row">
      <div class="segs" aria-hidden="true">
        {#each Array.from({ length: SEG }, (_, i) => i) as i (i)}
          <span class={i < n ? tone(i, p ?? 0) : ''}></span>
        {/each}
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
  .segs span.calm {
    background: var(--c-cyan);
  }
  .segs span.warm {
    background: var(--c-yellow);
  }
  .segs span.hot {
    background: var(--danger);
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
