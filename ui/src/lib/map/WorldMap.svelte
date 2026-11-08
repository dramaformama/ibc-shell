<script lang="ts">
  import {
    Focus,
    FolderOpen,
    Gauge,
    Globe,
    LocateFixed,
    MapPinOff,
    SquareTerminal,
    X
  } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import { flagName } from '$lib/flags/flags';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { serverStatus, toneRank, type StatusTone } from '$lib/stats/status';
  import { ms } from '$lib/stats/format';
  import { fitView, project, projectLonLat, spread, WORLD } from './geo';
  import { geo } from './geoip.svelte';
  import { BORDER_SCALE } from './borders';
  import { SMOOTH_OUTLINES as OUTLINES } from './outlines';
  import { CENTROIDS, GRID, LAND } from './world';
  import './world-map.css';

  /** Servers on a dot-matrix world map, one pin per country (from the server's flag). */
  let { servers }: { servers: Server[] } = $props();

  interface Pin {
    key: string;
    cc: string;
    /** City from the IP lookup; empty when placed by flag. */
    city: string;
    x: number;
    y: number;
    servers: Server[];
    tone: StatusTone;
    live: number;
  }

  /** By IP location when known (one pin per city), else by the server's flag (per country). */
  const groups = $derived.by(() => {
    const by = new Map<
      string,
      { key: string; cc: string; city: string; list: Server[]; at: [number, number] }
    >();
    for (const s of servers) {
      const hit = geo.get(s.host?.id);
      const cc = hit?.cc ?? s.flag?.toLowerCase();
      if (!cc) continue;
      const key = hit ? `@${hit.lat.toFixed(1)},${hit.lon.toFixed(1)}` : cc;
      const at = hit ? projectLonLat(hit.lon, hit.lat) : project(cc);
      if (!at) continue;
      const g = by.get(key) ?? { key, cc, city: hit?.city ?? '', list: [], at };
      g.list.push(s);
      by.set(key, g);
    }
    return [...by.values()];
  });
  // Look up hosts' locations when the option is on.
  $effect(() => {
    if (geo.enabled)
      void geo.locate(tree.data.hosts.map((h) => ({ id: h.id, hostname: h.hostname })));
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
        key: g.key,
        city: g.city,
        cc: g.cc,
        x: Math.max(1.5, Math.min(98.5, (placed[i]![0] / width) * 100)),
        y: Math.max(3, Math.min(97, (placed[i]![1] / height) * 100)),
        servers: g.list,
        tone: tones.sort((a, b) => toneRank[a] - toneRank[b])[0] ?? 'unknown',
        live: g.list.filter((s) => s.phase === 'connected').length
      };
    });
  });
  const unplaced = $derived(
    servers.filter((s) => !geo.get(s.host?.id) && (!s.flag || !CENTROIDS[s.flag.toLowerCase()]))
  );

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
  const openPin = $derived(pins.find((p) => p.key === open) ?? null);
  /** Countries that hold servers get filled in; the open pin's country stands out. */
  const lit = $derived(
    [...new Set(groups.map((g) => g.cc.toLowerCase()))].filter((c) => OUTLINES[c])
  );
  const openCc = $derived(openPin?.cc.toLowerCase() ?? null);
  const region = (cc: string) => flagName(cc, getLocale() === 'ru' ? 'ru' : 'en');
  const place = (p: Pin) => (p.city ? `${p.city}, ${region(p.cc)}` : region(p.cc));

  function go(s: Server, kind: 'terminal' | 'sftp' | 'status') {
    open = null;
    if (s.host) connectHost(s.host.id, kind);
    else if (s.group) sessions.select(s.group.current.id);
  }
</script>

<div class="map card">
  {#if !geo.enabled && servers.some((s) => s.host)}
    <div class="geo-offer">
      <LocateFixed size={16} />
      <p><b>{m.map_geo()}</b>{m.map_geo_hint()}</p>
      <button class="btn btn-sm btn-primary" onclick={() => geo.setEnabled(true)}
        >{m.map_geo_on()}</button
      >
    </div>
  {/if}
  <div class="canvas" style:aspect-ratio="{GRID.cols} / {GRID.rows}" bind:clientWidth={width}>
    <svg
      viewBox="{view.x - 0.5} {view.y - 0.5} {view.w} {view.h}"
      aria-hidden="true"
      preserveAspectRatio="none"
      style:--dot={0.62 / Math.pow(GRID.cols / view.w, 0.55)}
    >
      <path class="dots" d={LAND} />
      <g transform="scale({BORDER_SCALE})">
        <path class="borders" d={Object.values(OUTLINES).join('')} />
        {#each lit as c (c)}
          <path class="lit" class:on={c === openCc} d={OUTLINES[c]} />
        {/each}
      </g>
    </svg>
    {#each pins as p (p.key)}
      <button
        class="pin"
        data-tone={p.tone}
        class:active={open === p.key}
        style:left="{p.x}%"
        style:top="{p.y}%"
        onclick={() => (open = open === p.key ? null : p.key)}
        aria-label="{place(p)}: {p.servers.length}"
        title={place(p)}
        aria-expanded={open === p.key}
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
        aria-label={place(openPin)}
      >
        <header>
          <Flag code={openPin.cc} size={16} /><b>{place(openPin)}</b>
          <button class="mini" onclick={() => (open = null)} aria-label={m.action_close()}
            ><X size={13} /></button
          >
        </header>
        <ul>
          {#each openPin.servers.slice(0, 12) as s (s.id)}
            {@const st = serverStatus(s, false)}
            <li>
              <i class="dot" data-phase={st.tone}></i>
              <span class="nm"
                >{s.label}{#if s.folder}<em>{s.folder}</em>{/if}</span
              >
              <small class="tnum">{s.latency != null ? ms(s.latency) : st.text}</small>
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
    {#if geo.busy}<span class="unplaced">{m.map_geo_busy()}</span>{/if}
    {#if unplaced.length}
      <span class="unplaced" title={unplaced.map((s) => s.label).join(', ')}
        ><MapPinOff size={12} />{m.map_unplaced({ count: unplaced.length })}</span
      >
    {/if}
    <button
      class="btn btn-sm"
      aria-pressed={geo.enabled}
      onclick={() => geo.setEnabled(!geo.enabled)}
      title={m.map_geo_hint()}><LocateFixed size={13} />{m.map_geo()}</button
    >
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
