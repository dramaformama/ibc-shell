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

<style>
  .dashboard {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: 30px 32px 0;
    display: flex;
    flex-direction: column;
  }
  .heading {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 22px;
    flex: none;
  }
  h1 {
    margin: 0;
    font-size: 30px;
    line-height: 1.15;
    font-weight: 700;
    letter-spacing: -0.035em;
  }
  .subtitle {
    margin: 5px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .head-actions .seg button {
    height: 32px;
  }
  .recent {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 18px;
    flex: none;
  }
  .cap {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-right: 2px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .chip-srv {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    max-width: 220px;
    padding: 0 13px 0 9px;
    border: 0;
    border-radius: 999px;
    background: var(--surface);
    box-shadow: var(--card-shadow);
    color: var(--text);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
    transition:
      transform 120ms var(--ease),
      box-shadow 120ms var(--ease);
  }
  .chip-srv span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip-srv:hover {
    box-shadow:
      var(--card-shadow),
      0 4px 12px -6px rgb(0 0 0 / 0.2);
  }
  .chip-srv:active {
    transform: scale(0.97);
  }
  .summaries {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    flex: none;
    margin-bottom: 26px;
  }
  .summary {
    display: grid;
    grid-template-columns: 34px 1fr;
    align-items: center;
    gap: 2px 12px;
    border: 1.5px solid transparent;
    border-radius: var(--r-field);
    padding: 14px 16px;
    background: var(--surface);
    box-shadow: var(--card-shadow);
    text-align: left;
    cursor: pointer;
    font: inherit;
    color: var(--text);
    transition:
      border-color 150ms var(--ease),
      background-color 150ms var(--ease);
  }
  .summary:hover {
    background: color-mix(in oklab, var(--surface) 95%, var(--tone));
  }
  .summary[aria-pressed='true'] {
    border-color: var(--tone);
  }
  .summary-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 11px;
    background: color-mix(in oklab, var(--tone) 12%, transparent);
    color: var(--tone);
    grid-row: span 2;
  }
  .summary-label {
    font-size: 12px;
    color: var(--muted);
  }
  .summary b {
    font-size: 24px;
    font-weight: 650;
    line-height: 1.15;
    letter-spacing: -0.03em;
  }
  .sectionbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    margin-bottom: 14px;
    min-height: 34px;
    flex: none;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  h2 span {
    color: var(--muted);
    margin-left: 5px;
    font-size: 13px;
    font-weight: 500;
  }
  .live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 14px;
    margin-bottom: 28px;
    flex: none;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: var(--control-h);
    width: min(280px, 50%);
    border-radius: var(--r-control);
    color: var(--muted);
    background: var(--surface);
    box-shadow: var(--card-shadow);
  }
  .search:focus-within {
    box-shadow:
      0 0 0 3px var(--accent-soft),
      0 0 0 1px var(--accent);
  }
  input {
    border: 0;
    outline: 0;
    min-width: 0;
    width: 100%;
    font: inherit;
    background: none;
    color: var(--text);
  }
  input::placeholder {
    color: var(--muted);
  }
  footer {
    margin-top: auto;
    padding: 22px 0 18px;
    color: var(--muted);
    font-size: 11px;
    text-align: center;
    flex: none;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 32px;
    text-align: center;
    color: var(--muted);
  }
  .empty.small {
    flex: none;
    padding: 40px 16px;
  }
  .empty h2 {
    font-size: 20px;
    color: var(--text);
  }
  .empty p {
    margin: 0 0 8px;
    max-width: 40ch;
    text-wrap: pretty;
  }
  .empty .row {
    display: flex;
    gap: 10px;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    border-radius: 22px;
    background: var(--accent-soft);
    color: var(--accent);
    margin-bottom: 8px;
  }
  @media (max-width: 1100px) {
    .summaries {
      gap: 8px;
    }
    .summary {
      padding: 12px;
      grid-template-columns: 1fr;
    }
    .summary-icon {
      display: none;
    }
  }
  @media (max-width: 720px) {
    .dashboard {
      padding: 22px 16px 0;
    }
    .heading {
      flex-direction: column;
      align-items: stretch;
      gap: 14px;
    }
    .head-actions {
      justify-content: space-between;
    }
    h1 {
      font-size: 26px;
    }
    .summaries {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      margin-bottom: 20px;
    }
    .sectionbar {
      align-items: stretch;
      flex-direction: column;
      gap: 10px;
    }
    .search {
      width: 100%;
    }
  }
</style>
