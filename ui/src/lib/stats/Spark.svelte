<script lang="ts">
  /** Smooth multi-series line chart over the sample history (newest on the right). */
  interface Series {
    values: number[];
    color: string;
    fill?: boolean;
  }
  let { series, height = 64, max }: { series: Series[]; height?: number; max?: number } = $props();

  const W = 300;
  const top = $derived(
    Math.max(max ?? 0, ...series.flatMap((s) => s.values), 0.0001) * (max ? 1 : 1.15)
  );

  function path(vs: number[], close: boolean): string {
    if (vs.length < 2) return '';
    const step = W / (vs.length - 1);
    const pts = vs.map((v, i) => [i * step, height - (v / top) * (height - 4) - 2] as const);
    let d = `M${pts[0]![0]},${pts[0]![1]}`;
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1]!;
      const [x1, y1] = pts[i]!;
      const mx = (x0 + x1) / 2;
      d += ` C${mx},${y0} ${mx},${y1} ${x1},${y1}`;
    }
    return close ? `${d} L${W},${height} L0,${height} Z` : d;
  }
</script>

<svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" style:height="{height}px" aria-hidden="true">
  {#each [0.25, 0.5, 0.75] as g (g)}
    <line x1="0" x2={W} y1={height * g} y2={height * g} class="grid" />
  {/each}
  {#each series as s, i (i)}
    {#if s.fill}<path d={path(s.values, true)} style:fill={s.color} class="area" />{/if}
    <path d={path(s.values, false)} style:stroke={s.color} class="line" />
  {/each}
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
