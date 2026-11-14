<script lang="ts">
  import { edge } from '$lib/util/scroll-edge.svelte';
  import {
    CopyPlus,
    FolderOpen,
    Gauge,
    LayoutGrid,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    Plus,
    SquareTerminal,
    X
  } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { closeServer, connectAnother, switchKind } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { mod } from '$lib/util/platform';
  import WindowControls from './WindowControls.svelte';
  import { canOpenHostWindow, detachGroup, isHostWindow } from './host-window';
  import TabMenu from './TabMenu.svelte';
  import { TabDrag } from './tab-drag.svelte';

  let { mac = false, windowControls = false }: { mac?: boolean; windowControls?: boolean } =
    $props();

  const active = $derived(sessions.active);
  /** Servers connected for monitoring that have no tab. */
  const hidden = $derived(sessions.groups.length - sessions.visibleGroups.length);

  let tabsEl: HTMLDivElement;
  const drag = new TabDrag(
    () => [...tabsEl.querySelectorAll<HTMLElement>('[data-tab-key]')],
    (key, to) => sessions.moveGroup(key, to),
    () => tabsEl.getBoundingClientRect(),
    (key) => {
      if (!canOpenHostWindow || !sessions.groups.find((g) => g.key === key)?.current.hostId)
        return false;
      void detachGroup(key);
      return true;
    }
  );

  function onKey(e: KeyboardEvent, i: number) {
    const gs = sessions.visibleGroups;
    const n = gs.length;
    // ⌥← / ⌥→ moves the focused tab.
    if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      e.preventDefault();
      const g = gs[i];
      if (!g) return;
      sessions.moveGroup(g.key, i + (e.key === 'ArrowRight' ? 1 : -1));
      void Promise.resolve().then(() => document.getElementById('tab-' + g.key)?.focus());
      return;
    }
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

<div
  class="bar"
  class:mac
  class:edge={edge.scrolled || (active && active.kind !== 'status')}
  data-tauri-drag-region
>
  {#if !isHostWindow}
    <button
      class="icon-btn side-toggle"
      onclick={() => (ui.sidebarOpen = !ui.sidebarOpen)}
      aria-label={ui.sidebarOpen ? m.sidebar_hide() : m.sidebar_show()}
      aria-pressed={ui.sidebarOpen}
      title="{ui.sidebarOpen ? m.sidebar_hide() : m.sidebar_show()} ({mod('B')})"
      >{#if ui.sidebarOpen}<PanelLeftClose size={16} />{:else}<PanelLeftOpen
          size={16}
        />{/if}</button
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
  {/if}
  {#if isHostWindow}
    <button
      class="icon-btn"
      aria-label={m.dash_title()}
      title={m.dash_title()}
      onclick={() => {
        ui.page = 'overview';
        sessions.activeId = null;
      }}><LayoutGrid size={16} /></button
    >
  {/if}
  <div
    class="tabs"
    class:dragging={drag.key !== null}
    class:settling={drag.settling}
    role="tablist"
    aria-label={m.tabs_label()}
    data-tauri-drag-region
    bind:this={tabsEl}
  >
    {#each sessions.visibleGroups as g, i (g.key)}
      {@const on = g.key === sessions.activeGroup}
      <TabMenu group={g}>
        {#snippet children(props)}
          <div
            {...!isHostWindow ? props : {}}
            class="tab"
            class:active={on}
            class:lifted={drag.key === g.key}
            role="presentation"
            data-tab-key={g.key}
            style:transform={drag.offset(g.key)}
          >
            <button
              id={'tab-' + g.key}
              class="tab-main"
              class:host-window-tab={isHostWindow}
              role="tab"
              aria-selected={on}
              tabindex={on ? 0 : -1}
              onpointerdown={(e) => !isHostWindow && drag.down(e, g.key)}
              onclick={(e) => {
                if (drag.swallowClick()) e.preventDefault();
                else sessions.select(g.current.id);
              }}
              onkeydown={(e) => onKey(e, i)}
            >
              <span class="dot" data-phase={sessions.groupPhase(g)}></span>
              <Flag code={g.flag} size={14} />
              <span class="name">{g.label}</span>
              {#if !on && sessions.groupActivity(g)}
                <span class="unseen" title={m.tab_unseen()}></span>
              {/if}
            </button>
            {#if !isHostWindow}
              <button
                class="close"
                aria-label={m.tab_disconnect()}
                title={m.tab_disconnect()}
                onclick={() => closeServer(g.key)}><X size={12} /></button
              >
            {/if}
          </div>
        {/snippet}
      </TabMenu>
    {/each}
    {#if hidden > 0}
      <button
        class="bg-pill tnum"
        onclick={() => {
          ui.page = 'overview';
          sessions.activeId = null;
        }}
        title={m.tabs_background_hint()}>{m.tabs_background({ count: hidden })}</button
      >
    {/if}
    {#if !isHostWindow}
      <button
        class="icon-btn new"
        onclick={() => (ui.paletteOpen = true)}
        aria-label={m.tab_new()}
        title="{m.tab_new()} ({mod('T')})"><Plus size={16} /></button
      >
    {/if}
  </div>

  {#if active}
    <div class="tools">
      {#if active.hostId}
        <div class="seg" role="group" aria-label={m.tabs_label()}>
          <button
            aria-pressed={active.kind === 'status'}
            title={m.tab_status()}
            onclick={() => switchKind('status')}
            ><Gauge size={14} /><span class="lbl">{m.tab_status()}</span></button
          >
          <button
            aria-pressed={active.kind === 'terminal'}
            title={m.tab_terminal()}
            onclick={() => switchKind('terminal')}
            ><SquareTerminal size={14} /><span class="lbl">{m.tab_terminal()}</span></button
          >
          <button
            aria-pressed={active.kind === 'sftp'}
            title={m.tab_files()}
            onclick={() => switchKind('sftp')}
            ><FolderOpen size={14} /><span class="lbl">{m.tab_files()}</span></button
          >
        </div>
      {/if}
      {#if active.hostId && !isHostWindow}
        {@const id = active.hostId}
        <button
          class="icon-btn"
          onclick={() => connectAnother(id)}
          aria-label={m.menu_connect_another()}
          title="{m.menu_connect_another()} ({mod('⇧T').replace('Ctrl+⇧', 'Ctrl+Shift+')})"
          ><CopyPlus size={16} /></button
        >
      {/if}
      {#if active.kind === 'terminal' && !active.local}
        <button
          class="icon-btn"
          onclick={() => ui.toggleInspector()}
          aria-label={m.panel_toggle()}
          aria-pressed={ui.inspectorOpen}
          title={m.panel_toggle()}><PanelRight size={16} /></button
        >
      {/if}
    </div>
  {/if}
  {#if windowControls}
    <div class="window-drag" data-tauri-drag-region></div>
    <WindowControls />
  {/if}
</div>

<style>
  .window-drag {
    align-self: stretch;
    flex: 0 0 12px;
  }
  .bg-pill {
    flex: none;
    height: 22px;
    padding: 0 9px;
    border: 0;
    border-radius: 999px;
    background: var(--fill-2);
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-cap);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 120ms var(--ease);
  }
  .bg-pill:hover {
    background: color-mix(in oklab, var(--text) 12%, transparent);
    color: var(--text);
  }
  .bar {
    user-select: none;
    -webkit-user-select: none;
    container-type: inline-size;
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--toolbar-h);
    padding: 0 10px 0 8px;
    border-bottom: 0.5px solid transparent;
    background: var(--bg);
    transition: border-color 200ms var(--ease);
    flex: none;
  }
  .bar.edge {
    border-bottom-color: var(--sep);
  }
  .bar.mac {
    padding-left: 84px;
  }
  /* Narrow window: the view switcher keeps its icons, the tabs keep their names. */
  @container (max-width: 1080px) {
    .seg .lbl {
      display: none;
    }
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
    flex: 0 1 auto;
    min-width: 86px;
    max-width: 200px;
    height: var(--control-h);
    border-radius: 999px;
    color: var(--muted);
    transition:
      background-color 120ms var(--ease),
      color 120ms var(--ease);
  }
  .tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  /* Drag to reorder: the others slide out of the way, the lifted one follows the pointer. */
  .tabs.dragging .tab {
    transition: transform 160ms var(--ease);
  }
  .tabs.dragging .tab.lifted {
    z-index: 2;
    transition: none;
    background: var(--surface);
    box-shadow:
      var(--shadow),
      0 0 0 0.5px var(--border);
    cursor: grabbing;
  }
  .tabs.settling .tab {
    transition: none;
  }
  .unseen {
    flex: none;
    width: 6px;
    height: 6px;
    margin-left: -2px;
    border-radius: 50%;
    background: var(--accent);
  }
  .tab.active {
    background: var(--fill-2);
    color: var(--text);
  }
  .tab-main {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 100%;
    padding: 0 28px 0 11px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .host-window-tab {
    padding-right: 10px;
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
    border-radius: 999px;
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
    width: 0.5px;
    height: 16px;
    margin: 0 4px;
    background: var(--border);
  }
  .bar > .overview[aria-pressed='true'],
  .tools .icon-btn[aria-pressed='true'] {
    background: var(--sel);
    color: var(--text);
  }
  .side-toggle[aria-pressed='true'] {
    background: transparent;
    color: var(--muted);
  }
  .side-toggle[aria-pressed='true']:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
</style>
