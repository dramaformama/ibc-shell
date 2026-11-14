<script lang="ts">
  import { scrollEdge } from '$lib/util/scroll-edge.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import GettingStarted from '$lib/start/GettingStarted.svelte';
  import { start } from '$lib/start/start.svelte';
  import { onMount } from 'svelte';
  import { FolderPlus, List, Map as MapIcon, Plus, Search } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { fleet } from '$lib/fleet/fleet.svelte';
  import { filterFleet, type Availability } from '$lib/fleet/model';
  import WorldMap from '$lib/map/WorldMap.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import ServerList from './ServerList.svelte';
  import './dashboard.css';

  const VIEW_KEY = 'ibc.overview.view';
  let view = $state<'list' | 'map'>('list');
  try {
    if (localStorage.getItem(VIEW_KEY) === 'map') view = 'map';
  } catch {
    /* storage unavailable */
  }
  function setView(v: 'list' | 'map') {
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
  // Connected servers lead; otherwise the catalogue order (folders, then name) is kept.
  const shown = $derived(
    filterFleet(servers, query, filter).sort((x, y) => Number(!!y.group) - Number(!!x.group))
  );
  const online = $derived(servers.filter((s) => s.availability === 'online').length);
  const offline = $derived(servers.filter((s) => s.availability === 'offline').length);
  const connected = $derived(servers.filter((s) => s.phase === 'connected').length);
  const filters = $derived([
    { key: 'all' as const, label: m.dash_total(), count: servers.length, tone: null },
    { key: 'online' as const, label: m.dash_online(), count: online, tone: 'online' },
    { key: 'offline' as const, label: m.dash_offline(), count: offline, tone: 'offline' },
    {
      key: 'unknown' as const,
      label: m.dash_unknown(),
      count: servers.length - online - offline,
      tone: 'unknown'
    }
  ]);
  onMount(() => {
    const timer = setInterval(() => (fleet.now = Date.now()), 15_000);
    return () => clearInterval(timer);
  });
</script>

<section
  class="dashboard"
  aria-label={m.dash_title()}
  bind:this={scroller}
  use:scrollEdge={view === 'map'}
>
  <header class="heading">
    <div class="title">
      <h1>{m.dash_title()}</h1>
      <p class="subtitle tnum">{m.dash_counts({ total: servers.length, connected })}</p>
    </div>
    <div class="head-actions">
      {#if servers.length}
        <label class="search"
          ><Search size={14} /><input
            type="search"
            bind:value={query}
            placeholder={m.dash_search()}
            aria-label={m.dash_search()}
          /></label
        >
        <div class="seg" role="group" aria-label={m.dash_view()}>
          <button
            aria-pressed={view === 'list'}
            onclick={() => setView('list')}
            title={m.dash_view_list()}
            aria-label={m.dash_view_list()}><List size={15} /></button
          >
          <button
            aria-pressed={view === 'map'}
            onclick={() => setView('map')}
            title={m.dash_view_map()}
            aria-label={m.dash_view_map()}><MapIcon size={15} /></button
          >
        </div>
      {/if}
      <button
        class="icon-btn"
        onclick={() => ui.openFolderEditor(null)}
        aria-label={m.action_new_folder()}
        title={m.action_new_folder()}><FolderPlus size={17} /></button
      >
      <button class="btn btn-primary" onclick={() => ui.openHostEditor(null)}
        ><Plus size={15} />{m.action_new_host()}</button
      >
    </div>
  </header>

  {#if servers.length}
    {#if start.visible}<GettingStarted />{/if}

    <div class="filters" role="group" aria-label={m.dash_filter()}>
      {#each filters as f (f.key)}
        <button
          aria-pressed={filter === f.key}
          onclick={() => (filter = filter === f.key && f.key !== 'all' ? 'all' : f.key)}
        >
          {#if f.tone}<i class="dot" data-phase={f.tone}></i>{/if}{f.label}<span class="tnum"
            >{f.count}</span
          >
        </button>
      {/each}
    </div>

    {#if view === 'map'}
      <WorldMap servers={shown} />
    {:else if shown.length}
      <div class="list" role="table" aria-label={m.dash_all()} aria-rowcount={shown.length + 1}>
        <div class="list-head" role="row">
          <span role="columnheader">{m.dash_col_server()}</span>
          <span role="columnheader" class="c-addr">{m.dash_col_address()}</span>
          <span role="columnheader" class="c-folder">{m.dash_col_folder()}</span>
          <span role="columnheader" class="c-load">{m.dash_col_load()}</span>
          <span role="columnheader" class="c-ping">{m.dash_col_latency()}</span>
          <span role="columnheader" class="c-state">{m.dash_col_state()}</span>
          <span role="columnheader" class="c-acts"></span>
        </div>
        {#if scroller}<ServerList servers={shown} {scroller} />{/if}
      </div>
    {:else}
      <EmptyState icon={Search} title={m.dash_no_results()} hint={m.dash_no_results_hint()} />
    {/if}
    <footer>{m.dash_checked_note()}</footer>
  {:else if tree.loaded}
    <div class="empty onboarding">
      <Logo size={64} />
      <h2>{m.dash_empty_title()}</h2>
      <p>{m.dash_empty_hint()}</p>
      {#if start.visible}
        <GettingStarted roomy />
      {:else}
        <div class="row">
          <button class="btn btn-primary btn-prominent" onclick={() => ui.openHostEditor(null)}
            ><Plus size={16} />{m.action_new_host()}</button
          >
          <button class="btn btn-prominent" onclick={() => ui.openFolderEditor(null)}
            ><FolderPlus size={16} />{m.action_new_folder()}</button
          >
          <button class="btn btn-prominent" onclick={() => (ui.paletteOpen = true)}
            >{m.dash_quick_connect()}</button
          >
        </div>
      {/if}
    </div>
  {/if}
</section>
