<script lang="ts">
  import { FolderPlus, Lock, PanelLeft, Plus, Search, Settings } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { mod } from '$lib/util/platform';
  import OpenSessions from './OpenSessions.svelte';
  import Tree from './Tree.svelte';

  let { mac = false }: { mac?: boolean } = $props();
</script>

<aside class="side" aria-label={m.sidebar_hosts()}>
  <div class="top" class:mac data-tauri-drag-region>
    <button
      class="icon-btn"
      onclick={() => (ui.sidebarOpen = false)}
      aria-label={m.sidebar_hide()}
      title="{m.sidebar_hide()} ({mod('B')})"
    >
      <PanelLeft size={16} />
    </button>
  </div>
  <button class="search" onclick={() => (ui.paletteOpen = true)}>
    <Search size={14} /><span>{m.sidebar_search()}</span><span class="kbd">{mod('K')}</span>
  </button>
  <OpenSessions />
  <h2 class="sect">{m.sidebar_hosts_section()}</h2>
  <Tree />
  <footer>
    <button
      class="icon-btn"
      onclick={() => ui.openHostEditor(null)}
      aria-label={m.action_new_host()}
      title={m.action_new_host()}><Plus size={16} /></button
    >
    <button
      class="icon-btn"
      onclick={() => ui.openFolderEditor(null)}
      aria-label={m.action_new_folder()}
      title={m.action_new_folder()}><FolderPlus size={16} /></button
    >
    <span class="sp"></span>
    <button
      class="icon-btn"
      onclick={() => (ui.settingsOpen = true)}
      aria-label={m.settings_title()}
      title="{m.settings_title()} ({mod(',')})"><Settings size={16} /></button
    >
    <button
      class="icon-btn"
      onclick={() => void vault.lock()}
      aria-label={m.action_lock()}
      title="{m.action_lock()} ({mod('L')})"><Lock size={16} /></button
    >
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
    justify-content: flex-start;
    height: 40px;
    padding: 0 8px;
    flex: none;
  }
  .top.mac {
    padding-left: 78px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin: 0 8px 10px;
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
  .sect {
    margin: 4px 0 2px;
    padding: 0 16px;
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
</style>
