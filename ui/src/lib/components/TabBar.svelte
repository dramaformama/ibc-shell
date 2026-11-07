<script lang="ts">
  import { PanelLeft, Plus, X } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { mod } from '$lib/util/platform';

  let { mac = false }: { mac?: boolean } = $props();

  function onKey(e: KeyboardEvent, i: number) {
    const n = sessions.tabs.length;
    const to = e.key === 'ArrowRight' ? (i + 1) % n : e.key === 'ArrowLeft' ? (i - 1 + n) % n : -1;
    if (to < 0) return;
    e.preventDefault();
    const t = sessions.tabs[to];
    if (t) {
      sessions.activeId = t.id;
      document.getElementById('tab-' + t.id)?.focus();
    }
  }
</script>

<div class="bar" class:mac={mac && !ui.sidebarOpen} data-tauri-drag-region>
  {#if !ui.sidebarOpen}
    <button
      class="icon-btn"
      onclick={() => (ui.sidebarOpen = true)}
      aria-label={m.sidebar_show()}
      title="{m.sidebar_show()} ({mod('B')})"><PanelLeft size={16} /></button
    >
  {/if}
  <div class="tabs" role="tablist" aria-label={m.tabs_label()}>
    {#each sessions.tabs as t, i (t.id)}
      <div class="tab" class:active={t.id === sessions.activeId} role="presentation">
        <button
          id={'tab-' + t.id}
          class="tab-main"
          role="tab"
          aria-selected={t.id === sessions.activeId}
          tabindex={t.id === sessions.activeId ? 0 : -1}
          onclick={() => (sessions.activeId = t.id)}
          onkeydown={(e) => onKey(e, i)}
        >
          <Flag code={t.flag} size={14} />
          <span class="name">{t.label}</span>
          {#if t.kind === 'sftp'}<span class="tag">SFTP</span>{/if}
        </button>
        <button class="close" aria-label={m.tab_close()} onclick={() => sessions.close(t.id)}
          ><X size={12} /></button
        >
      </div>
    {/each}
  </div>
  <button
    class="icon-btn"
    onclick={() => (ui.paletteOpen = true)}
    aria-label={m.tab_new()}
    title="{m.tab_new()} ({mod('T')})"><Plus size={16} /></button
  >
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 40px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex: none;
  }
  .bar.mac {
    padding-left: 78px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    display: flex;
    align-items: center;
    flex: none;
    max-width: 200px;
    height: 28px;
    border-radius: var(--r-control);
    color: var(--muted);
    transition: background-color 120ms var(--ease);
  }
  .tab:hover {
    background: var(--surface-2);
  }
  .tab.active {
    background: var(--surface-2);
    color: var(--text);
  }
  .tab-main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 100%;
    padding: 0 4px 0 10px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    font-size: 10px;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .close {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-right: 4px;
    border: 0;
    border-radius: 4px;
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
    background: var(--border);
  }
</style>
