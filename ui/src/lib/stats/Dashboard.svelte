<script lang="ts">
  import { onMount } from 'svelte';
  import {
    CircleHelp,
    History,
    LayoutGrid,
    Map as MapIcon,
    Plus,
    Search,
    Server,
    Wifi,
    WifiOff
  } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import Flag from '$lib/flags/Flag.svelte';
  import { fleet } from '$lib/fleet/fleet.svelte';
  import { filterFleet, type Availability } from '$lib/fleet/model';
  import WorldMap from '$lib/map/WorldMap.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { recent } from '$lib/stores/recent.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import ServerCard from './ServerCard.svelte';
  import ServerGrid from './ServerGrid.svelte';
  import './dashboard.css';

  const VIEW_KEY = 'ibc.overview.view';
  let view = $state<'cards' | 'map'>('cards');
  try {
    if (localStorage.getItem(VIEW_KEY) === 'map') view = 'map';
  } catch {
    /* storage unavailable */
  }
  function setView(v: 'cards' | 'map') {
    view = v;
    try {
      localStorage.setItem(VIEW_KEY, v);
    } catch {
      /* storage unavailable */
    }
  }

  let query = $state('');
  let filter = $state<'all' | Availability>('all');
  let scroller = $state<HTMLElement | null>(null);
  const servers = $derived(fleet.servers);
  const shown = $derived(filterFleet(servers, query, filter));
  const live = $derived(shown.filter((s) => s.group));
  const rest = $derived(shown.filter((s) => !s.group));
  const online = $derived(servers.filter((s) => s.availability === 'online').length);
  const offline = $derived(servers.filter((s) => s.availability === 'offline').length);
  const connected = $derived(servers.filter((s) => s.phase === 'connected').length);
  const recents = $derived(
    recent
      .prune((id) => !!tree.host(id))
      .slice(0, 6)
      .map((e) => servers.find((s) => s.id === e.id))
      .filter((s) => !!s)
  );
  const summaries = $derived([
    {
      key: 'all' as const,
      label: m.dash_total(),
      count: servers.length,
      icon: Server,
      tone: 'var(--accent)'
    },
    {
      key: 'online' as const,
      label: m.dash_online(),
      count: online,
      icon: Wifi,
      tone: 'var(--ok)'
    },
    {
      key: 'offline' as const,
      label: m.dash_offline(),
      count: offline,
      icon: WifiOff,
      tone: 'var(--danger)'
    },
    {
      key: 'unknown' as const,
      label: m.dash_unknown(),
      count: servers.length - online - offline,
      icon: CircleHelp,
      tone: 'var(--muted)'
    }
  ]);
  onMount(() => {
    const timer = setInterval(() => (fleet.now = Date.now()), 15_000);
    return () => clearInterval(timer);
  });
</script>

<section class="dashboard" aria-label={m.dash_title()} bind:this={scroller}>
  <header class="heading">
    <div>
      <h1>{m.dash_title()}</h1>
      <p class="subtitle tnum">
        {m.dash_counts({ total: servers.length, connected })}
      </p>
    </div>
    <div class="head-actions">
      {#if servers.length}
        <div class="seg" role="group" aria-label={m.dash_view()}>
          <button aria-pressed={view === 'cards'} onclick={() => setView('cards')}
            ><LayoutGrid size={14} />{m.dash_view_cards()}</button
          >
          <button aria-pressed={view === 'map'} onclick={() => setView('map')}
            ><MapIcon size={14} />{m.dash_view_map()}</button
          >
        </div>
      {/if}
      <button class="btn btn-primary btn-prominent" onclick={() => ui.openHostEditor(null)}
        ><Plus size={16} />{m.action_new_host()}</button
      >
    </div>
  </header>

  {#if servers.length}
    {#if recents.length}
      <div class="recent" aria-label={m.dash_recent()}>
        <span class="cap"><History size={13} />{m.dash_recent()}</span>
        {#each recents as s (s.id)}
          <button
            class="chip-srv"
            onclick={() => s.host && connectHost(s.host.id)}
            title={s.address}
          >
            <Flag code={s.flag} size={15} /><span>{s.label}</span>
            {#if s.phase === 'connected'}<i class="dot" data-phase="connected"></i>{/if}
          </button>
        {/each}
      </div>
    {/if}

    <div class="summaries" role="group" aria-label={m.dash_filter()}>
      {#each summaries as item (item.key)}
        <button
          class="summary"
          aria-pressed={filter === item.key}
          onclick={() => (filter = filter === item.key ? 'all' : item.key)}
          style:--tone={item.tone}
        >
          <span class="summary-icon"><item.icon size={17} /></span>
          <span class="summary-label">{item.label}</span><b class="tnum">{item.count}</b>
        </button>
      {/each}
    </div>

    {#if view === 'map'}
      <div class="sectionbar">
        <h2>{m.dash_view_map()} <span class="tnum">{shown.length}</span></h2>
        <label class="search"
          ><Search size={15} /><input
            type="search"
            bind:value={query}
            placeholder={m.dash_search()}
            aria-label={m.dash_search()}
          /></label
        >
      </div>
      <WorldMap servers={shown} />
    {:else}
      {#if live.length}
        <div class="sectionbar">
          <h2>{m.dash_live()} <span class="tnum">{live.length}</span></h2>
        </div>
        <div class="live-grid">
          {#each live as s (s.id)}<ServerCard server={s} />{/each}
        </div>
      {/if}
      <div class="sectionbar">
        <h2>
          {live.length ? m.dash_others() : m.dash_all()} <span class="tnum">{rest.length}</span>
        </h2>
        <label class="search"
          ><Search size={15} /><input
            type="search"
            bind:value={query}
            placeholder={m.dash_search()}
            aria-label={m.dash_search()}
          /></label
        >
      </div>
      {#if rest.length && scroller}
        <ServerGrid servers={rest} {scroller} />
      {:else if !shown.length}
        <div class="empty small">
          <Search size={24} />
          <h2>{m.dash_no_results()}</h2>
          <p>{m.dash_no_results_hint()}</p>
        </div>
      {/if}
    {/if}
    <footer>{m.dash_checked_note()}</footer>
  {:else if tree.loaded}
    <div class="empty onboarding">
      <div class="empty-icon"><Server size={30} /></div>
      <h2>{m.dash_empty_title()}</h2>
      <p>{m.dash_empty_hint()}</p>
      <div class="row">
        <button class="btn btn-primary btn-prominent" onclick={() => ui.openHostEditor(null)}
          ><Plus size={16} />{m.action_new_host()}</button
        >
        <button class="btn btn-prominent" onclick={() => (ui.paletteOpen = true)}
          >{m.dash_quick_connect()}</button
        >
      </div>
    </div>
  {/if}
</section>
