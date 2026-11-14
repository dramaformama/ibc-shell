<script lang="ts">
  /**
   * Placeholder rows while data loads: the shape of what is coming, with a soft shimmer,
   * instead of a spinner. `label` is what screen readers hear.
   */
  let { rows = 4, label, two = true }: { rows?: number; label: string; two?: boolean } = $props();
  // Fixed, varied widths: real lists are ragged, identical bars read as a progress bar.
  const W = [62, 48, 71, 55, 66, 43, 58, 69];
</script>

<div class="sk" role="status" aria-label={label}>
  {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
    <div class="row" aria-hidden="true" style:--d="{i * 90}ms">
      <i class="dot"></i>
      <span class="lines">
        <i style:width="{W[i % W.length]}%"></i>
        {#if two}<i class="sub" style:width="{W[(i + 3) % W.length]! * 0.6}%"></i>{/if}
      </span>
      <i class="end"></i>
    </div>
  {/each}
</div>

<style>
  .sk {
    display: grid;
    gap: 4px;
    padding: 4px 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 4px;
  }
  .lines {
    flex: 1;
    display: grid;
    gap: 6px;
  }
  i {
    display: block;
    height: 9px;
    border-radius: 5px;
    background: linear-gradient(
        100deg,
        transparent 20%,
        color-mix(in oklab, var(--bg) 70%, transparent) 50%,
        transparent 80%
      )
      var(--fill-2);
    background-size: 300% 100%;
    animation: shimmer 1.6s var(--ease) infinite;
    animation-delay: var(--d);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .sub {
    height: 7px;
    opacity: 0.7;
  }
  .end {
    width: 44px;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
</style>
