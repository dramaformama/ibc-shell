<script lang="ts">
  import { onMount } from 'svelte';
  import type { Server } from '$lib/fleet/model';
  import ServerCard from './ServerCard.svelte';

  let { servers }: { servers: Server[] } = $props();
  let viewport: HTMLDivElement;
  let width = $state(900);
  let height = $state(600);
  let scrollTop = $state(0);
  const STEP = 304;
  const columns = $derived(Math.max(1, Math.floor((width + 16) / 336)));
  const rowCount = $derived(Math.ceil(servers.length / columns));
  const first = $derived(Math.max(0, Math.min(rowCount - 1, Math.floor(scrollTop / STEP)) - 1));
  const last = $derived(Math.min(rowCount, first + Math.ceil(height / STEP) + 3));
  const visible = $derived(servers.slice(first * columns, last * columns));

  onMount(() => {
    const resize = new ResizeObserver(([entry]) => {
      if (!entry) return;
      width = entry.contentRect.width;
      height = entry.contentRect.height;
    });
    resize.observe(viewport);
    return () => resize.disconnect();
  });
</script>

<div class="viewport" bind:this={viewport} onscroll={() => (scrollTop = viewport.scrollTop)}>
  <div class="canvas" style:height="{rowCount * STEP}px" role="list">
    <div
      class="visible"
      style:transform="translateY({first * STEP}px)"
      style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
    >
      {#each visible as server, i (server.id)}
        <div role="listitem" aria-setsize={servers.length} aria-posinset={first * columns + i + 1}>
          <ServerCard {server} />
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .viewport {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 2px 2px 0;
  }
  .canvas {
    position: relative;
  }
  .visible {
    position: absolute;
    inset: 0 0 auto;
    display: grid;
    gap: 16px;
  }
</style>
