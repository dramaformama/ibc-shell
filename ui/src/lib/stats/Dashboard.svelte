<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, CircleHelp, Search, Server, Plus, Wifi, WifiOff } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { fleet } from '$lib/fleet/fleet.svelte';
  import { filterFleet, type Availability } from '$lib/fleet/model';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import ServerGrid from './ServerGrid.svelte';

  let query = $state('');
  let filter = $state<'all' | Availability>('all');
  const servers = $derived(fleet.servers);
  const shown = $derived(filterFleet(servers, query, filter));
  const online = $derived(servers.filter((s) => s.availability === 'online').length);
  const offline = $derived(servers.filter((s) => s.availability === 'offline').length);
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
    const timer = setInterval(() => (fleet.now = Date.now()), 5000);
    return () => clearInterval(timer);
  });
</script>

<section class="dashboard" aria-label={m.dash_title()}>
  <header class="heading">
    <div>
      <p class="eyebrow">{m.app_name()}</p>
      <h1>{m.dash_title()}</h1>
      <p class="subtitle">{m.dash_subtitle()}</p>
    </div>
    <button class="btn btn-primary btn-prominent" onclick={() => ui.openHostEditor(null)}
      ><Plus size={16} />{m.action_new_host()}</button
    >
  </header>
  {#if servers.length}
    <div class="summaries" role="group" aria-label={m.dash_filter()}>
      {#each summaries as item (item.key)}
        <button
          class="summary"
          aria-pressed={filter === item.key}
          onclick={() => (filter = item.key)}
          style:--tone={item.tone}
        >
          <span class="summary-icon"><item.icon size={17} /></span>
          <span class="summary-label">{item.label}</span><b class="tnum">{item.count}</b>
        </button>
      {/each}
    </div>
    <div class="sectionbar">
      <h2>{m.dash_all()} <span class="tnum">{shown.length}</span></h2>
      <label class="search"
        ><Search size={15} /><input
          type="search"
          bind:value={query}
          placeholder={m.dash_search()}
          aria-label={m.dash_search()}
        /></label
      >
    </div>
    {#if shown.length}
      <ServerGrid servers={shown} />
    {:else}
      <div class="empty">
        <Search size={28} />
        <h2>{m.dash_no_results()}</h2>
        <p>{m.dash_no_results_hint()}</p>
      </div>
    {/if}
    <footer><Activity size={12} /><span>{m.dash_checked_note()}</span></footer>
  {:else if tree.loaded}
    <div class="empty onboarding">
      <div class="empty-icon"><Server size={30} /></div>
      <h2>{m.dash_empty_title()}</h2>
      <p>{m.dash_empty_hint()}</p>
      <button class="btn btn-primary btn-prominent" onclick={() => ui.openHostEditor(null)}
        ><Plus size={16} />{m.action_new_host()}</button
      >
    </div>
  {/if}
</section>

<style>
  .dashboard {
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 32px 32px 0;
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
    flex: none;
  }
  .eyebrow {
    margin: 0 0 4px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  h1 {
    margin: 0;
    font-size: 32px;
    line-height: 1.2;
    font-weight: 700;
    letter-spacing: -0.035em;
  }
  .subtitle {
    margin: 7px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .summaries {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    flex: none;
    margin-bottom: 28px;
  }
  .summary {
    display: grid;
    grid-template-columns: 30px 1fr;
    align-items: center;
    gap: 4px 10px;
    border: 1px solid transparent;
    border-radius: 12px;
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
    background: color-mix(in oklab, var(--surface) 96%, var(--tone));
  }
  .summary[aria-pressed='true'] {
    border-color: color-mix(in oklab, var(--tone) 40%, transparent);
  }
  .summary-icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: color-mix(in oklab, var(--tone) 11%, transparent);
    color: var(--tone);
    grid-row: span 2;
  }
  .summary-label {
    font-size: 11px;
    color: var(--muted);
  }
  .summary b {
    font-size: 23px;
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: -0.03em;
  }
  .sectionbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
    flex: none;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.015em;
  }
  h2 span {
    color: var(--muted);
    margin-left: 6px;
    font-size: 12px;
    font-weight: 400;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    height: 32px;
    width: min(280px, 50%);
    border: 1px solid var(--border);
    border-radius: var(--r-control);
    color: var(--muted);
    background: var(--surface);
  }
  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
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
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--muted);
    padding: 14px 0;
    font-size: 11px;
    flex: none;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px;
    text-align: center;
    color: var(--muted);
  }
  .empty h2 {
    font-size: 22px;
    color: var(--text);
  }
  .empty p {
    margin: 0 0 8px;
    max-width: 38ch;
    text-wrap: pretty;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    border-radius: 20px;
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
      padding: 24px 16px 0;
    }
    .heading {
      margin-bottom: 20px;
    }
    h1 {
      font-size: 28px;
    }
    .summaries {
      margin-bottom: 20px;
    }
    .summary {
      padding: 10px;
    }
    .heading > button {
      font-size: 12px;
      padding: 0 10px;
    }
    .sectionbar {
      align-items: start;
      flex-direction: column;
      gap: 12px;
    }
    .search {
      width: 100%;
    }
    footer {
      text-align: center;
      font-size: 10px;
    }
  }
</style>
