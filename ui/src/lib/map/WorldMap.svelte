<script lang="ts">
  import { Focus, FolderOpen, Gauge, Globe, MapPinOff, SquareTerminal, X } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import { flagName } from '$lib/flags/flags';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { serverStatus, toneRank, type StatusTone } from '$lib/stats/status';
  import { fitView, project, spread, WORLD } from './geo';
  import { CENTROIDS, GRID, LAND } from './world';
  import './world-map.css';

  /** Servers on a dot-matrix world map, one pin per country (from the server's flag). */
  let { servers }: { servers: Server[] } = $props();

  interface Pin {
    cc: string;
    x: number;
    y: number;
    servers: Server[];
    tone: StatusTone;
    live: number;
  }

  const groups = $derived.by(() => {
    const by = new Map<string, Server[]>();
    for (const s of servers) {
      const cc = s.flag?.toLowerCase();
      if (cc && CENTROIDS[cc]) by.set(cc, [...(by.get(cc) ?? []), s]);
    }
    return [...by].map(([cc, list]) => ({ cc, list, at: project(cc)! }));
  });

  /** Zoomed to the servers by default; "Whole world" shows everything. */
  let whole = $state(false);
  const view = $derived(whole ? WORLD : fitView(groups.map((g) => g.at)));
  let width = $state(1000);
  const height = $derived((width * GRID.rows) / GRID.cols);

  const pins = $derived.by<Pin[]>(() => {
    // Busiest countries keep their true spot; neighbours step aside.
    const sorted = [...groups].sort((a, b) => b.list.length - a.list.length);
    const px = sorted.map((g): [number, number] => [
      ((g.at[0] - view.x) / view.w) * width,
      ((g.at[1] - view.y) / view.h) * height
    ]);
    const placed = spread(px, 34);
    return sorted.map((g, i) => {
      const tones = g.list.map((s) => serverStatus(s, false).tone);
      return {
        cc: g.cc,
        x: Math.max(1.5, Math.min(98.5, (placed[i]![0] / width) * 100)),
        y: Math.max(3, Math.min(97, (placed[i]![1] / height) * 100)),
        servers: g.list,
        tone: tones.sort((a, b) => toneRank[a] - toneRank[b])[0] ?? 'unknown',
        live: g.list.filter((s) => s.phase === 'connected').length
      };
    });
  });
  const unplaced = $derived(servers.filter((s) => !s.flag || !CENTROIDS[s.flag.toLowerCase()]));

  // Availability for the pins (bounded: the first 100 saved, not connected servers).
  $effect(() => {
    if (!settings.probe) return;
    const stops = servers
      .filter((s) => s.host && s.phase !== 'connected')
      .slice(0, 100)
      .map((s) => reachability.watch(s.host!.id));
    return () => stops.forEach((f) => f());
  });

  let open = $state<string | null>(null);
  const openPin = $derived(pins.find((p) => p.cc === open) ?? null);
  const region = (cc: string) => flagName(cc, getLocale() === 'ru' ? 'ru' : 'en');

  function go(s: Server, kind: 'terminal' | 'sftp' | 'status') {
    open = null;
    if (s.host) connectHost(s.host.id, kind);
    else if (s.group) sessions.select(s.group.current.id);
  }
</script>

<div class="map card">
  <div class="canvas" style:aspect-ratio="{GRID.cols} / {GRID.rows}" bind:clientWidth={width}>
    <svg
      viewBox="{view.x - 0.5} {view.y - 0.5} {view.w} {view.h}"
      aria-hidden="true"
      preserveAspectRatio="none"
      style:--dot={0.62 / Math.pow(GRID.cols / view.w, 0.55)}
    >
      <path d={LAND} />
    </svg>
    {#each pins as p (p.cc)}
      <button
        class="pin"
        data-tone={p.tone}
        class:active={open === p.cc}
        style:left="{p.x}%"
        style:top="{p.y}%"
        onclick={() => (open = open === p.cc ? null : p.cc)}
        aria-label="{region(p.cc)}: {p.servers.length}"
        aria-expanded={open === p.cc}
      >
        {#if p.live}<span class="pulse"></span>{/if}
        <span class="head"><Flag code={p.cc} size={18} /></span>
        {#if p.servers.length > 1}<span class="n tnum">{p.servers.length}</span>{/if}
      </button>
    {/each}
    {#if openPin}
      <div
        class="pop"
        style:left="{Math.min(Math.max(openPin.x, 14), 86)}%"
        style:top="{openPin.y}%"
        class:below={openPin.y < 45}
        role="dialog"
        aria-label={region(openPin.cc)}
      >
        <header>
          <Flag code={openPin.cc} size={16} /><b>{region(openPin.cc)}</b>
          <button class="mini" onclick={() => (open = null)} aria-label={m.action_close()}
            ><X size={13} /></button
          >
        </header>
        <ul>
          {#each openPin.servers.slice(0, 12) as s (s.id)}
            {@const st = serverStatus(s, false)}
            <li>
              <i class="dot" data-phase={st.tone}></i>
              <span class="nm">{s.label}</span>
              <small class="tnum">{s.latency != null ? `${s.latency} ms` : st.text}</small>
              <button
                class="mini"
                onclick={() => go(s, 'status')}
                title={m.tab_status()}
                aria-label={m.tab_status()}><Gauge size={13} /></button
              >
              <button
                class="mini"
                onclick={() => go(s, 'sftp')}
                title={m.tab_files()}
                aria-label={m.tab_files()}><FolderOpen size={13} /></button
              >
              <button
                class="mini"
                onclick={() => go(s, 'terminal')}
                title={m.tab_terminal()}
                aria-label={m.tab_terminal()}><SquareTerminal size={13} /></button
              >
            </li>
          {/each}
        </ul>
        {#if openPin.servers.length > 12}
          <p class="more">+{openPin.servers.length - 12}</p>
        {/if}
      </div>
    {/if}
  </div>
  <footer>
    <span class="key"><i data-tone="connected"></i>{m.status_connected()}</span>
    <span class="key"><i data-tone="online"></i>{m.dash_available()}</span>
    <span class="key"><i data-tone="offline"></i>{m.dash_unavailable()}</span>
    <span class="key"><i data-tone="unknown"></i>{m.dash_not_checked()}</span>
    <span class="sp"></span>
    {#if unplaced.length}
      <span class="unplaced" title={unplaced.map((s) => s.label).join(', ')}
        ><MapPinOff size={12} />{m.map_unplaced({ count: unplaced.length })}</span
      >
    {/if}
    {#if groups.length}
      <button
        class="btn btn-sm"
        onclick={() => (whole = !whole)}
        aria-pressed={whole}
        title={whole ? m.map_fit() : m.map_world()}
        >{#if whole}<Focus size={13} />{m.map_fit()}{:else}<Globe
            size={13}
          />{m.map_world()}{/if}</button
      >
    {/if}
  </footer>
</div>
