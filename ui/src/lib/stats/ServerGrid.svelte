<script lang="ts">
  import { onMount } from 'svelte';
  import type { Server } from '$lib/fleet/model';
  import ServerTile from './ServerTile.svelte';

  /**
   * Virtualized grid of compact server tiles. It scrolls with `scroller` (the overview's own
   * scroll area), rendering only the rows near the viewport, so 5000 servers stay cheap.
   */
  let { servers, scroller }: { servers: Server[]; scroller: HTMLElement } = $props();
  let grid: HTMLDivElement;
  let width = $state(900);
  let viewH = $state(800);
  let scrollTop = $state(0);
  let offset = $state(0);
  const ROW = 68;
  const GAP = 10;
  const STEP = ROW + GAP;
  const columns = $derived(Math.max(1, Math.floor((width + GAP) / (300 + GAP))));
  const rowCount = $derived(Math.ceil(servers.length / columns));
  const first = $derived(Math.max(0, Math.floor((scrollTop - offset) / STEP) - 2));
  const last = $derived(Math.min(rowCount, first + Math.ceil(viewH / STEP) + 5));
  const visible = $derived(servers.slice(first * columns, last * columns));

  onMount(() => {
    const measure = () => {
      offset =
        grid.getBoundingClientRect().top -
        scroller.getBoundingClientRect().top +
        scroller.scrollTop;
      viewH = scroller.clientHeight;
    };
    // Sections above the grid can grow (live cards appear), so the offset is re-read on scroll.
    const onScroll = () => {
      measure();
      scrollTop = scroller.scrollTop;
    };
    const ro = new ResizeObserver(([e]) => {
      if (e) width = e.contentRect.width;
      measure();
    });
    ro.observe(grid);
    ro.observe(scroller);
    scroller.addEventListener('scroll', onScroll, { passive: true });
    measure();
    return () => {
      ro.disconnect();
      scroller.removeEventListener('scroll', onScroll);
    };
  });
</script>

<div
  class="canvas"
  bind:this={grid}
  style:height="{Math.max(0, rowCount * STEP - GAP)}px"
  role="list"
>
  <div
    class="rows"
    style:transform="translateY({first * STEP}px)"
    style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
  >
    {#each visible as server, i (server.id)}
      <div role="listitem" aria-setsize={servers.length} aria-posinset={first * columns + i + 1}>
        <ServerTile {server} />
      </div>
    {/each}
  </div>
</div>

<style>
  .canvas {
    position: relative;
    flex: none;
  }
  .rows {
    position: absolute;
    inset: 0 0 auto;
    display: grid;
    gap: 10px;
  }
</style>
