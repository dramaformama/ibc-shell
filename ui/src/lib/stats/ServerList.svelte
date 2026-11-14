<script lang="ts">
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { backOut } from 'svelte/easing';
  import type { Server } from '$lib/fleet/model';
  import { settings } from '$lib/stores/settings.svelte';
  import HoverCard from './HoverCard.svelte';
  import ServerRow from './ServerRow.svelte';

  /**
   * Virtualized server rows. It scrolls with `scroller` (the overview's own scroll area) and
   * renders only the rows near the viewport, so 5000 servers stay cheap.
   */
  let { servers, scroller }: { servers: Server[]; scroller: HTMLElement } = $props();
  let list: HTMLDivElement;
  let viewH = $state(800);
  let scrollTop = $state(0);
  let offset = $state(0);
  // Must match `--row-tile` in app.css for each density.
  const ROW = $derived(settings.density === 'compact' ? 32 : 40);
  const first = $derived(Math.max(0, Math.floor((scrollTop - offset) / ROW) - 4));
  const last = $derived(Math.min(servers.length, first + Math.ceil(viewH / ROW) + 8));
  const visible = $derived(servers.slice(first, last));

  // Quick look: rest the pointer on a row; moving into the card keeps it open.
  let hover = $state<{ id: string; rect: DOMRect } | null>(null);
  const hovered = $derived(hover ? servers.find((s) => s.id === hover!.id) : undefined);
  let timer: ReturnType<typeof setTimeout> | undefined;
  function enter(id: string, e: PointerEvent) {
    if (e.pointerType !== 'mouse') return;
    const el = e.currentTarget as HTMLElement;
    clearTimeout(timer);
    timer = setTimeout(() => (hover = { id, rect: el.getBoundingClientRect() }), hover ? 120 : 650);
  }
  function leave() {
    clearTimeout(timer);
    timer = setTimeout(() => (hover = null), 160);
  }
  const keep = () => clearTimeout(timer);

  onMount(() => {
    const measure = () => {
      offset =
        list.getBoundingClientRect().top -
        scroller.getBoundingClientRect().top +
        scroller.scrollTop;
      viewH = scroller.clientHeight;
    };
    const onScroll = () => {
      clearTimeout(timer);
      hover = null;
      measure();
      scrollTop = scroller.scrollTop;
    };
    const ro = new ResizeObserver(measure);
    ro.observe(list);
    ro.observe(scroller);
    scroller.addEventListener('scroll', onScroll, { passive: true });
    measure();
    return () => {
      ro.disconnect();
      scroller.removeEventListener('scroll', onScroll);
    };
  });
</script>

<div class="canvas" bind:this={list} style:height="{servers.length * ROW}px" role="rowgroup">
  <div class="rows" style:transform="translateY({first * ROW}px)">
    {#each visible as server (server.id)}
      <!-- A server that connects glides to the top; the others make room. -->
      <div
        animate:flip={{ duration: 420, easing: backOut }}
        role="presentation"
        onpointerenter={(e) => enter(server.id, e)}
        onpointerleave={leave}
      >
        <ServerRow {server} />
      </div>
    {/each}
  </div>
</div>

{#if hover && hovered}
  <HoverCard server={hovered} rect={hover.rect} onenter={keep} onleave={leave} />
{/if}

<style>
  .canvas {
    position: relative;
    flex: none;
  }
  .rows {
    position: absolute;
    inset: 0 0 auto;
  }
</style>
