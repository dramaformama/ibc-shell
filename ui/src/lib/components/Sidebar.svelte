<script lang="ts">
  import {
    Box,
    FolderPlus,
    LayoutGrid,
    Lock,
    Plus,
    Search,
    Settings,
    ShieldCheck,
    Zap
  } from '@lucide/svelte';
  import { connectedServers } from '$lib/fleetpages/connected.svelte';
  import { services } from '$lib/services/services.svelte';
  import { certTone, daysLeft } from '$lib/services/ports';
  import { snippets } from '$lib/snippets/snippets.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { mod } from '$lib/util/platform';
  import Logo from './Logo.svelte';
  import Tree from './Tree.svelte';

  let { mac = false }: { mac?: boolean } = $props();

  const running = $derived(
    Object.values(services.docker).reduce(
      (n, r) => n + r.containers.filter((c) => c.state === 'running').length,
      0
    )
  );
  // The counters must not wait for the Docker / Domains pages to be opened once.
  const liveSessions = $derived(connectedServers().map((x) => x.sid));
  $effect(() => {
    const stops = liveSessions.flatMap((sid) => [
      services.watch('docker', sid),
      services.watch('certs', sid)
    ]);
    return () => stops.forEach((stop) => stop());
  });
  const domains = $derived(Object.values(services.certs).reduce((n, r) => n + r.certs.length, 0));
  const expiring = $derived(
    Object.values(services.certs).reduce(
      (n, r) => n + r.certs.filter((c) => certTone(daysLeft(c.not_after)) !== 'ok').length,
      0
    )
  );
  const pages = $derived([
    {
      id: 'overview' as const,
      label: m.dash_title(),
      icon: LayoutGrid,
      count: tree.data.hosts.length,
      alert: false
    },
    { id: 'docker' as const, label: m.dk_title(), icon: Box, count: running, alert: false },
    {
      id: 'domains' as const,
      label: m.fleet_domains(),
      icon: ShieldCheck,
      count: domains,
      alert: expiring > 0
    },
    {
      id: 'snippets' as const,
      label: m.panel_snippets(),
      icon: Zap,
      count: snippets.items.length,
      alert: false
    }
  ]);
</script>

<aside class="side" aria-label={m.sidebar_hosts()}>
  <div class="top" class:mac data-tauri-drag-region>
    <button class="search" onclick={() => (ui.paletteOpen = true)}>
      <Search size={14} /><span>{m.sidebar_search()}</span><span class="kbd">{mod('K')}</span>
    </button>
  </div>
  <nav aria-label={m.dash_title()}>
    {#each pages as pg (pg.id)}
      <button
        class="nav-item"
        aria-current={sessions.activeId === null && ui.page === pg.id ? 'page' : undefined}
        onclick={() => {
          ui.page = pg.id;
          sessions.activeId = null;
        }}
      >
        <pg.icon size={17} /><span>{pg.label}</span>
        {#if pg.count}<span class="nav-count tnum" class:alert={pg.alert}>{pg.count}</span>{/if}
      </button>
    {/each}
  </nav>
  <div class="section-heading">
    <h2 class="sect">{m.sidebar_hosts_section()}</h2>
    <button
      class="icon-btn"
      onclick={() => ui.openHostEditor(null)}
      aria-label={m.action_new_host()}
      title={m.action_new_host()}><Plus size={14} /></button
    >
    <button
      class="icon-btn"
      onclick={() => ui.openFolderEditor(null)}
      aria-label={m.action_new_folder()}
      title={m.action_new_folder()}><FolderPlus size={14} /></button
    >
  </div>
  <Tree />
  <footer>
    <span class="brand"><Logo size={18} />iBC Shell</span>
    <span class="sp"></span>
    <button
      class="icon-btn"
      onclick={() => (ui.settingsOpen = true)}
      aria-label={m.settings_title()}
      title="{m.settings_title()} ({mod(',')})"><Settings size={16} /></button
    >
    {#if vault.canLock}
      <button
        class="icon-btn"
        onclick={() => void vault.lock()}
        aria-label={m.action_lock()}
        title="{m.action_lock()} ({mod('L')})"><Lock size={16} /></button
      >
    {/if}
  </footer>
</aside>

<style>
  .side {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-right: 1px solid var(--border);
    background: var(--sidebar);
  }
  .top {
    display: flex;
    align-items: center;
    height: var(--toolbar-h);
    padding: 0 8px;
    flex: none;
  }
  .top.mac {
    padding-left: 84px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    height: var(--control-h);
    padding: 0 6px 0 10px;
    border: 1px solid transparent;
    border-radius: var(--r-control);
    background: var(--hover);
    color: var(--muted);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
    transition: border-color 120ms var(--ease);
  }
  .search:hover {
    border-color: color-mix(in oklab, var(--muted) 50%, var(--border));
  }
  .search span:first-of-type {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
  }
  nav {
    padding: 12px 10px 16px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-control);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .nav-item > :global(svg) {
    color: var(--accent);
  }
  .nav-item:hover {
    background: var(--hover);
  }
  .nav-item[aria-current='page'] {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .nav-count {
    margin-left: auto;
    font-size: 11px;
    color: var(--muted);
  }
  .nav-count.alert {
    min-width: 18px;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--warn) 18%, transparent);
    color: color-mix(in oklab, var(--warn) 70%, var(--text));
    font-weight: 600;
    text-align: center;
  }
  nav {
    display: grid;
    gap: 2px;
  }
  .section-heading {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0 12px 6px 18px;
  }
  .section-heading .icon-btn {
    width: 24px;
    height: 24px;
  }
  .sect {
    margin: 0;
    padding: 0;
    flex: 1;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 8px calc(6px + env(safe-area-inset-bottom));
    border-top: 1px solid var(--border);
  }
  .sp {
    flex: 1;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding-left: 4px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: -0.005em;
    user-select: none;
  }
</style>
