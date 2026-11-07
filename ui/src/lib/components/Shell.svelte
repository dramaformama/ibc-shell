<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { api } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { isApple } from '$lib/util/platform';
  import CommandPalette from './CommandPalette.svelte';
  import ConfirmDelete from './ConfirmDelete.svelte';
  import FolderEditor from './FolderEditor.svelte';
  import HostEditor from './HostEditor.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import Sidebar from './Sidebar.svelte';
  import TabBar from './TabBar.svelte';
  import Toasts from './Toasts.svelte';

  let { children }: { children: Snippet } = $props();

  let mac = $state(false);
  let narrow = $state(false);

  onMount(() => {
    void api.appInfo().then((r) => r.status === 'ok' && (mac = r.data.platform === 'macos'));
    const mq = matchMedia('(max-width: 720px)');
    const sync = () => {
      if (mq.matches && !narrow) ui.sidebarOpen = false;
      narrow = mq.matches;
    };
    sync();
    mq.addEventListener('change', sync);
    return () => mq.removeEventListener('change', sync);
  });

  function onKeyDown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      sessions.step(e.shiftKey ? -1 : 1);
      return;
    }
    if (!(isApple ? e.metaKey : e.ctrlKey) || e.altKey) return;
    const k = e.key.toLowerCase();
    if (e.shiftKey && k !== 'tab') return;
    if (k === 'k') ui.paletteOpen = !ui.paletteOpen;
    else if (k === 'b') ui.sidebarOpen = !ui.sidebarOpen;
    else if (k === 'l') void vault.lock();
    else if (k === ',') ui.settingsOpen = true;
    else if (k === 't') ui.paletteOpen = true;
    else if (k === 'w' && sessions.activeId) sessions.close(sessions.activeId);
    else if (/^[1-9]$/.test(k)) {
      const t = sessions.tabs[Number(k) - 1];
      if (t) sessions.activeId = t.id;
    } else return;
    e.preventDefault();
  }

  // Sidebar resize: pointer drag on the 4 px handle, 200–420 px.
  function startResize(e: PointerEvent) {
    e.preventDefault();
    const move = (ev: PointerEvent) => (ui.sidebarWidth = Math.min(420, Math.max(200, ev.clientX)));
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      settings.saveSidebarWidth();
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }
  function resizeKey(e: KeyboardEvent) {
    const d = e.key === 'ArrowLeft' ? -16 : e.key === 'ArrowRight' ? 16 : 0;
    if (!d) return;
    ui.sidebarWidth = Math.min(420, Math.max(200, ui.sidebarWidth + d));
    settings.saveSidebarWidth();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="shell" class:narrow style:--sw="{ui.sidebarOpen && !narrow ? ui.sidebarWidth : 0}px">
  <div
    class="sidebar"
    class:open={ui.sidebarOpen}
    inert={!ui.sidebarOpen}
    style:width="{ui.sidebarWidth}px"
  >
    <Sidebar {mac} />
    {#if !narrow}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="grip"
        role="separator"
        aria-orientation="vertical"
        aria-label={m.sidebar_resize()}
        aria-valuenow={ui.sidebarWidth}
        aria-valuemin="200"
        aria-valuemax="420"
        tabindex="0"
        onpointerdown={startResize}
        onkeydown={resizeKey}
      ></div>
    {/if}
  </div>
  {#if narrow && ui.sidebarOpen}
    <button class="scrim" aria-label={m.action_close()} onclick={() => (ui.sidebarOpen = false)}
    ></button>
  {/if}
  <div class="main">
    <TabBar {mac} />
    <main class="content">{@render children()}</main>
  </div>
</div>

{#if ui.editor?.kind === 'host'}
  {#key ui.editor.id ?? 'new'}<HostEditor id={ui.editor.id} folderId={ui.editor.folderId} />{/key}
{:else if ui.editor?.kind === 'folder'}
  {#key ui.editor.id ?? 'new'}<FolderEditor id={ui.editor.id} parentId={ui.editor.parentId} />{/key}
{/if}
<CommandPalette />
<SettingsDialog />
<ConfirmDelete />
<Toasts />

<style>
  .shell {
    position: relative;
    display: grid;
    grid-template-columns: var(--sw) minmax(0, 1fr);
    height: 100%;
    overflow: hidden;
  }
  .sidebar {
    position: relative;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
  }
  .sidebar.open {
    visibility: visible;
  }
  .grip {
    position: absolute;
    top: 0;
    right: -2px;
    bottom: 0;
    width: 5px;
    cursor: col-resize;
    z-index: 5;
  }
  .grip:hover,
  .grip:focus-visible {
    background: var(--accent);
    opacity: 0.4;
    outline: none;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    height: 100%;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .narrow {
    grid-template-columns: minmax(0, 1fr);
  }
  .narrow .sidebar {
    position: fixed;
    inset: 0 auto 0 0;
    z-index: 40;
    width: min(320px, 86vw) !important;
    transform: translateX(-100%);
    transition:
      transform 150ms var(--ease),
      visibility 0s linear 150ms;
    box-shadow: var(--shadow);
  }
  .narrow .sidebar.open {
    transform: none;
    transition-delay: 0s;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 30;
    border: 0;
    background: color-mix(in oklab, var(--text) 18%, transparent);
  }
</style>
