<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  /** Fixed-row-height virtual list: only visible rows (plus overscan) are in the DOM. */
  let {
    items,
    rowHeight,
    key,
    row,
    empty,
    scroller = $bindable(null),
    overscan = 8
  }: {
    items: readonly T[];
    rowHeight: number;
    key: (item: T) => string;
    row: Snippet<[T, number]>;
    empty?: Snippet;
    scroller?: HTMLDivElement | null;
    overscan?: number;
  } = $props();

  let top = $state(0);
  let height = $state(0);

  const first = $derived(Math.max(0, Math.floor(top / rowHeight) - overscan));
  const last = $derived(Math.min(items.length, Math.ceil((top + height) / rowHeight) + overscan));
  const slice = $derived(items.slice(first, last));

  /** Scrolls so that row `i` is fully visible. */
  export function reveal(i: number): void {
    if (!scroller) return;
    const y = i * rowHeight;
    if (y < scroller.scrollTop) scroller.scrollTop = y;
    else if (y + rowHeight > scroller.scrollTop + scroller.clientHeight)
      scroller.scrollTop = y + rowHeight - scroller.clientHeight;
  }
</script>

<div
  class="vl"
  bind:this={scroller}
  bind:clientHeight={height}
  onscroll={(e) => (top = e.currentTarget.scrollTop)}
>
  {#if items.length === 0}
    {@render empty?.()}
  {:else}
    <div class="space" style:height="{items.length * rowHeight}px">
      <div class="win" style:transform="translateY({first * rowHeight}px)">
        {#each slice as item, i (key(item))}
          <div class="slot" style:height="{rowHeight}px">{@render row(item, first + i)}</div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .vl {
    flex: 1;
    min-height: 0;
    overflow: auto;
    position: relative;
  }
  .space {
    position: relative;
  }
  .win {
    position: absolute;
    inset: 0 0 auto 0;
  }
  .slot {
    display: block;
  }
</style>
