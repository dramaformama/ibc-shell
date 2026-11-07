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

<style>
  .map {
    position: relative;
    padding: 18px 18px 12px;
  }
  .canvas {
    position: relative;
    width: 100%;
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  path {
    stroke: color-mix(in oklab, var(--muted) 34%, transparent);
    stroke-width: var(--dot);
    stroke-linecap: round;
    vector-effect: none;
  }
  svg {
    transition: none;
  }
  .sp {
    flex: 1;
  }
  .pin {
    --c: var(--muted);
    position: absolute;
    transition:
      left 300ms var(--ease),
      top 300ms var(--ease);
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin: -14px 0 0 -14px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    cursor: pointer;
    z-index: 1;
  }
  .pin[data-tone='connected'],
  .key i[data-tone='connected'] {
    --c: var(--ok);
  }
  .pin[data-tone='online'],
  .key i[data-tone='online'] {
    --c: var(--accent);
  }
  .pin[data-tone='offline'],
  .pin[data-tone='failed'],
  .key i[data-tone='offline'] {
    --c: var(--danger);
  }
  .pin[data-tone='connecting'] {
    --c: var(--warn);
  }
  .head {
    position: absolute;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--surface);
    box-shadow:
      0 0 0 2.5px var(--c),
      0 4px 12px -2px rgb(0 0 0 / 0.25);
    transition: transform 160ms var(--ease);
  }
  .pin:hover .head,
  .pin.active .head,
  .pin:focus-visible .head {
    transform: scale(1.15);
  }
  .pin:hover,
  .pin.active {
    z-index: 3;
  }
  .n {
    position: absolute;
    top: -8px;
    right: -10px;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--c);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    line-height: 17px;
    text-align: center;
    box-shadow: 0 0 0 2px var(--surface);
  }
  .pulse {
    position: absolute;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--c);
    opacity: 0.35;
    animation: ping 2.4s var(--ease) infinite;
  }
  @keyframes ping {
    from {
      transform: scale(1);
      opacity: 0.4;
    }
    to {
      transform: scale(2.4);
      opacity: 0;
    }
  }
  .pop {
    position: absolute;
    z-index: 5;
    width: 300px;
    max-width: 92%;
    padding: 10px;
    border-radius: var(--r-field);
    background: var(--surface);
    box-shadow:
      var(--shadow),
      0 0 0 0.5px var(--border);
    transform: translate(-50%, calc(-100% - 24px));
    animation: pop 160ms var(--ease);
  }
  .pop.below {
    transform: translate(-50%, 24px);
  }
  @keyframes pop {
    from {
      opacity: 0;
      scale: 0.96;
    }
  }
  .pop header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 2px 8px 4px;
  }
  .pop header b {
    flex: 1;
    font-size: 13px;
  }
  .pop ul {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .pop li {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 2px 0 6px;
    border-radius: 10px;
  }
  .pop li:hover {
    background: var(--hover);
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .pop small {
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .dot[data-phase='online'] {
    background: var(--accent);
  }
  .dot[data-phase='offline'] {
    background: var(--danger);
  }
  .mini {
    display: grid;
    place-items: center;
    flex: none;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .mini:hover {
    background: var(--surface-2);
    color: var(--accent);
  }
  .more {
    margin: 6px 6px 0;
    color: var(--muted);
    font-size: 11px;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 16px;
    margin-top: 12px;
    color: var(--muted);
    font-size: 11px;
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .key i {
    --c: var(--muted);
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--surface);
    box-shadow: 0 0 0 2px var(--c);
  }
  .unplaced {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
</style>
