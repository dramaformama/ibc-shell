<script lang="ts">
  import {
    Box,
    FolderPlus,
    Keyboard,
    LayoutGrid,
    Lock,
    Plus,
    Search,
    CircleArrowUp,
    Settings,
    ShieldCheck,
    SquareTerminal,
    Zap
  } from '@lucide/svelte';
  import { connectedServers } from '$lib/fleetpages/connected.svelte';
  import { services } from '$lib/services/services.svelte';
  import { certTone, daysLeft } from '$lib/services/ports';
  import { snippets } from '$lib/snippets/snippets.svelte';
  import { api } from '$lib/api';
  import { settings } from '$lib/stores/settings.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { untrack } from 'svelte';
  import { hkey } from '$lib/util/tree-model';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { mod } from '$lib/util/platform';
  import Logo from './Logo.svelte';
  import Tree from './Tree.svelte';

  let { mac = false }: { mac?: boolean } = $props();

  // The counters must not wait for the Docker / Domains pages to be opened once.
  const liveSessions = $derived(connectedServers().map((x) => x.sid));

  // The tree follows the open tab: switching tabs selects that server and opens its folders.
  const activeHost = $derived(sessions.active?.hostId ?? null);
  $effect(() => {
    const id = tree.loaded ? activeHost : null;
    const host = id ? untrack(() => tree.host(id)) : null;
    if (!host) return;
    untrack(() => {
      const key = hkey(host.id);
      if (tree.selected.size !== 1 || !tree.selected.has(key)) tree.select([key], key);
      else tree.cursor = key;
      for (let p = host.folder_id; p;) {
        const f = tree.folder(p);
        if (!f) break;
        if (!f.expanded) void tree.setExpanded(f.id, true);
        p = f.parent_id;
      }
    });
  });
  // Only live sessions count: reports of closed or reconnected sessions stay in the store and
  // would double the numbers the pages themselves show.
  const running = $derived(
    liveSessions.reduce(
      (n, sid) =>
        n + (services.docker[sid]?.containers.filter((c) => c.state === 'running').length ?? 0),
      0
    )
  );
  $effect(() => {
    const stops = liveSessions.flatMap((sid) => [
      services.watch('docker', sid),
      services.watch('certs', sid)
    ]);
    return () => stops.forEach((stop) => stop());
  });
  const liveCerts = $derived(liveSessions.flatMap((sid) => services.certs[sid]?.certs ?? []));
  const domains = $derived(liveCerts.length);
  const expiring = $derived(
    liveCerts.filter((c) => certTone(daysLeft(c.not_after)) !== 'ok').length
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
        <pg.icon size={16} /><span>{pg.label}</span>
        {#if pg.count}<span class="nav-count tnum" class:alert={pg.alert}>{pg.count}</span>{/if}
      </button>
    {/each}
    {#if sessions.localTab}
      {@const local = sessions.localTab}
      <button
        class="nav-item"
        aria-current={sessions.activeId === local.id ? 'page' : undefined}
        title={m.local_terminal() + ' (⌃`)'}
        onclick={() => sessions.select(local.id)}
      >
        <SquareTerminal size={16} /><span>{m.local_terminal()}</span>
        {#if sessions.activity[local.id]}<span class="nav-unseen" title={m.tab_unseen()}
          ></span>{/if}
      </button>
    {/if}
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
  {#if settings.update}
    {@const u = settings.update}
    <button
      type="button"
      class="update"
      title={m.update_open()}
      onclick={() => void api.openExternal(u.url)}
    >
      <CircleArrowUp size={14} /><span>{m.update_available({ version: u.version })}</span>
    </button>
  {/if}
  <footer>
    <span class="brand"><Logo size={18} />iBC Shell</span>
    <span class="sp"></span>
    <button
      class="icon-btn"
      onclick={() => (ui.hotkeysOpen = true)}
      aria-label={m.keys_title()}
      title="{m.keys_title()} (?)"><Keyboard size={16} /></button
    >
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
  /* Floating source-list panel (macOS 26): inset from the window edge, glass material. */
  .side {
    height: calc(100% - 16px);
    margin: 8px 0 8px 8px;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-radius: var(--r-panel);
    background: var(--sidebar);
    -webkit-backdrop-filter: var(--glass-blur);
    backdrop-filter: var(--glass-blur);
    box-shadow:
      0 0 0 0.5px var(--sidebar-edge),
      0 8px 24px -12px rgb(0 0 0 / 0.14);
    overflow: hidden;
  }
  .top {
    display: flex;
    align-items: center;
    height: calc(var(--toolbar-h) - 8px);
    padding: 0 8px;
    flex: none;
  }
  .top.mac {
    padding-left: 76px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    height: var(--control-h);
    padding: 0 5px 0 8px;
    border: 0;
    border-radius: var(--r-control);
    background: var(--fill-2);
    color: var(--muted);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 120ms var(--ease);
  }
  .search:hover {
    background: color-mix(in oklab, var(--text) 10%, transparent);
  }
  .search span:first-of-type {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
  }
  nav {
    display: grid;
    gap: 1px;
    padding: 6px 8px 14px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--r-inner);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .nav-item > :global(svg) {
    color: var(--accent);
    stroke-width: 1.75;
  }
  .nav-item:hover {
    background: var(--hover);
  }
  .nav-item[aria-current='page'] {
    background: var(--sel);
    font-weight: 600;
  }
  .nav-count {
    margin-left: auto;
    font-size: var(--fs-cap);
    color: var(--muted);
    font-weight: 400;
  }
  .nav-unseen {
    width: 7px;
    height: 7px;
    margin-left: auto;
    margin-right: 4px;
    border-radius: 50%;
    background: var(--accent);
  }
  .nav-count.alert {
    color: var(--warn);
    font-weight: 600;
  }
  .nav-count.alert::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: 5px;
    border-radius: 50%;
    background: var(--warn);
    vertical-align: 1px;
  }
  .section-heading {
    display: flex;
    align-items: center;
    padding: 0 8px 4px 16px;
  }
  .section-heading .icon-btn {
    width: 22px;
    height: 22px;
    opacity: 0;
    transition: opacity 120ms var(--ease);
  }
  .side:hover .section-heading .icon-btn,
  .section-heading .icon-btn:focus-visible {
    opacity: 1;
  }
  .sect {
    margin: 0;
    padding: 0;
    flex: 1;
    font-size: var(--fs-cap);
    font-weight: 600;
    color: var(--muted);
  }
  .update {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 8px 2px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--r-inner);
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: var(--fs-sm);
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .update:hover {
    background: var(--hover);
  }
  .update span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 6px 8px calc(6px + env(safe-area-inset-bottom));
    border-top: 0.5px solid var(--sep);
  }
  footer .icon-btn {
    width: 26px;
    height: 26px;
  }
  .sp {
    flex: 1;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding-left: 4px;
    color: var(--muted);
    font-size: var(--fs-sm);
    font-weight: 600;
    user-select: none;
  }
</style>
