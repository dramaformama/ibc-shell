<script lang="ts">
  import { loadColor, pct } from './format';

  /** Circular gauge with the value in the middle (CPU / RAM). */
  let {
    value,
    size = 64,
    label
  }: { value: number | null; size?: number; label: string } = $props();

  const r = $derived(size / 2 - 5);
  const c = $derived(2 * Math.PI * r);
  const color = $derived(loadColor(value));
</script>

<div
  class="gauge-ring"
  style:width="{size}px"
  style:height="{size}px"
  role="img"
  aria-label="{label} {pct(value)}%"
>
  <svg viewBox="0 0 {size} {size}" width={size} height={size} aria-hidden="true">
    <circle cx={size / 2} cy={size / 2} {r} class="track" style:stroke={color} />
    <circle
      cx={size / 2}
      cy={size / 2}
      {r}
      class="bar"
      style:stroke={color}
      stroke-dasharray={c}
      stroke-dashoffset={c * (1 - Math.min(100, Math.max(0, value ?? 0)) / 100)}
      transform="rotate(-90 {size / 2} {size / 2})"
    />
  </svg>
  <span class="v tnum" style:font-size="{Math.round(size * 0.24)}px">{pct(value)}%</span>
</div>

<style>
  .gauge-ring {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
  }
  svg {
    position: absolute;
    inset: 0;
  }
  circle {
    fill: none;
    stroke-width: 6;
    stroke-linecap: round;
  }
  .track {
    opacity: 0.18;
  }
  .bar {
    transition:
      stroke-dashoffset 600ms var(--ease),
      stroke 300ms var(--ease);
  }
  .v {
    font-weight: 700;
    letter-spacing: -0.02em;
  }
</style>
