<script lang="ts">
  import {
    Activity,
    FolderOpen,
    Gauge,
    LayoutGrid,
    Zap,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
    Power,
    SquareTerminal,
    X
  } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { switchKind } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { mod } from '$lib/util/platform';

  let { mac = false }: { mac?: boolean } = $props();

  const active = $derived(sessions.active);

  function onKey(e: KeyboardEvent, i: number) {
    const gs = sessions.groups;
    const n = gs.length;
    const to = e.key === 'ArrowRight' ? (i + 1) % n : e.key === 'ArrowLeft' ? (i - 1 + n) % n : -1;
    if (to < 0) return;
    e.preventDefault();
    const g = gs[to];
    if (g) {
      sessions.select(g.current.id);
      document.getElementById('tab-' + g.key)?.focus();
    }
  }
</script>

<div class="bar" class:mac={mac && !ui.sidebarOpen} data-tauri-drag-region>
  <button
    class="icon-btn"
    onclick={() => (ui.sidebarOpen = !ui.sidebarOpen)}
    aria-label={ui.sidebarOpen ? m.sidebar_hide() : m.sidebar_show()}
    aria-pressed={ui.sidebarOpen}
    title="{ui.sidebarOpen ? m.sidebar_hide() : m.sidebar_show()} ({mod('B')})"
    >{#if ui.sidebarOpen}<PanelLeftClose size={16} />{:else}<PanelLeftOpen size={16} />{/if}</button
  >
  {#if !ui.sidebarOpen}
    <button
      class="icon-btn overview"
      onclick={() => (sessions.activeId = null)}
      aria-label={m.dash_title()}
      aria-pressed={sessions.activeId === null}
      title={m.dash_title()}><LayoutGrid size={16} /></button
    >
  {/if}
  <span class="sep" aria-hidden="true"></span>
  <div class="tabs" role="tablist" aria-label={m.tabs_label()} data-tauri-drag-region>
    {#each sessions.groups as g, i (g.key)}
      {@const on = g.key === sessions.activeGroup}
      <div class="tab" class:active={on} role="presentation">
        <button
          id={'tab-' + g.key}
          class="tab-main"
          role="tab"
          aria-selected={on}
          tabindex={on ? 0 : -1}
          onclick={() => sessions.select(g.current.id)}
          onkeydown={(e) => onKey(e, i)}
        >
          <span class="dot" data-phase={sessions.groupPhase(g)}></span>
          <Flag code={g.flag} size={14} />
          <span class="name">{g.label}</span>
        </button>
        <button
          class="close"
          aria-label={m.tab_disconnect()}
          title={m.tab_disconnect()}
          onclick={() => sessions.closeGroup(g.key)}><X size={12} /></button
        >
      </div>
    {/each}
    <button
      class="icon-btn new"
      onclick={() => (ui.paletteOpen = true)}
      aria-label={m.tab_new()}
      title="{m.tab_new()} ({mod('T')})"><Plus size={16} /></button
    >
  </div>

  {#if active}
    <div class="tools">
      {#if active.hostId}
        <div class="seg" role="group" aria-label={m.tabs_label()}>
          <button aria-pressed={active.kind === 'status'} onclick={() => switchKind('status')}
            ><Gauge size={14} />{m.tab_status()}</button
          >
          <button aria-pressed={active.kind === 'terminal'} onclick={() => switchKind('terminal')}
            ><SquareTerminal size={14} />{m.tab_terminal()}</button
          >
          <button aria-pressed={active.kind === 'sftp'} onclick={() => switchKind('sftp')}
            ><FolderOpen size={14} />{m.tab_files()}</button
          >
        </div>
      {/if}
      {#if active.kind === 'terminal'}
        <span class="sep" aria-hidden="true"></span>
        <button
          class="icon-btn"
          onclick={() => ui.showPanel('metrics')}
          aria-label={m.panel_metrics()}
          aria-pressed={ui.inspectorOpen && ui.panelTab === 'metrics'}
          title={m.panel_metrics()}><Activity size={16} /></button
        >
        <button
          class="icon-btn"
          onclick={() => ui.showPanel('snippets')}
          aria-label={m.panel_snippets()}
          aria-pressed={ui.inspectorOpen && ui.panelTab === 'snippets'}
          title={m.panel_snippets()}><Zap size={16} /></button
        >
      {/if}
      <button
        class="icon-btn"
        onclick={() => sessions.activeGroup && sessions.closeGroup(sessions.activeGroup)}
        aria-label={m.tab_disconnect()}
        title="{m.tab_disconnect()} ({mod('W')})"><Power size={15} /></button
      >
    </div>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: var(--toolbar-h);
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    flex: none;
  }
  .bar.mac {
    padding-left: 78px;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
    max-width: 220px;
    height: 30px;
    border-radius: var(--r-control);
    color: var(--muted);
    transition:
      background-color 120ms var(--ease),
      color 120ms var(--ease);
  }
  .tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tab.active {
    background: var(--surface);
    color: var(--text);
    box-shadow:
      var(--card-shadow),
      0 0 0 0.5px var(--border);
  }
  .tab-main {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 100%;
    padding: 0 30px 0 10px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .tab.active .tab-main {
    font-weight: 500;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    position: absolute;
    right: 5px;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: inherit;
    opacity: 0;
    cursor: pointer;
  }
  .tab:hover .close,
  .tab.active .close,
  .close:focus-visible {
    opacity: 1;
  }
  .close:hover {
    background: var(--hover);
  }
  .new {
    flex: none;
  }
  .sep {
    flex: none;
    width: 1px;
    height: 16px;
    margin: 0 2px;
    background: var(--border);
  }
  .bar > .overview[aria-pressed='true'],
  .tools .icon-btn[aria-pressed='true'] {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
</style>
