<script lang="ts">
  /** Smooth multi-series line chart over the sample history (newest on the right). */
  interface Series {
    values: (number | null)[];
    color: string;
    fill?: boolean;
  }
  let { series, height = 64, max }: { series: Series[]; height?: number; max?: number } = $props();

  const W = 300;
  /** A line needs two points; before that the chart is a quiet baseline, not an empty grid. */
  const hasData = $derived(series.some((s) => s.values.filter((v) => v !== null).length >= 2));
  const top = $derived(
    Math.max(
      max ?? 0,
      ...series.flatMap((s) => s.values.filter((v): v is number => v !== null)),
      0.0001
    ) * (max ? 1 : 1.15)
  );

  function path(vs: (number | null)[], close: boolean): string {
    if (vs.length < 2) return '';
    const step = W / (vs.length - 1);
    const parts: string[] = [];
    let segment = '';
    let first = 0;
    let prev: [number, number] | null = null;
    const finish = () => {
      if (prev && segment)
        parts.push(close ? `${segment} L${prev[0]},${height} L${first},${height} Z` : segment);
      prev = null;
      segment = '';
    };
    vs.forEach((v, i) => {
      if (v == null) {
        finish();
        return;
      }
      const x = i * step;
      const y = height - (v / top) * (height - 4) - 2;
      if (prev) {
        const mx = (prev[0] + x) / 2;
        segment += ` C${mx},${prev[1]} ${mx},${y} ${x},${y}`;
      } else {
        first = x;
        segment = `M${x},${y}`;
      }
      prev = [x, y];
    });
    finish();
    return parts.join(' ');
  }
</script>

<svg
  viewBox="0 0 {W} {height}"
  preserveAspectRatio="none"
  style:height="{height}px"
  aria-hidden="true"
>
  {#if hasData}
    {#each [0.25, 0.5, 0.75] as g (g)}
      <line x1="0" x2={W} y1={height * g} y2={height * g} class="grid" />
    {/each}
    {#each series as s, i (i)}
      {#if s.fill}<path d={path(s.values, true)} style:fill={s.color} class="area" />{/if}
      <path d={path(s.values, false)} style:stroke={s.color} class="line" />
    {/each}
  {:else}
    <line x1="0" x2={W} y1={height - 1} y2={height - 1} class="base" />
  {/if}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    overflow: visible;
  }
  .grid {
    stroke: var(--border);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    stroke-dasharray: 2 3;
  }
  .base {
    stroke: var(--border);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .line {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  .area {
    opacity: 0.12;
  }
</style>
