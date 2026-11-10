<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { api } from '$lib/api';
  import { closeServer, connectAnother } from '$lib/stores/actions';
  import * as m from '$lib/paraglide/messages.js';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { isApple, shortcutKey } from '$lib/util/platform';
  import CloseServerDialog from './CloseServerDialog.svelte';
  import HotkeysDialog from '$lib/help/HotkeysDialog.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import ConfirmDelete from './ConfirmDelete.svelte';
  import HostKeyDialog from '$lib/terminal/HostKeyDialog.svelte';
  import FolderEditor from './FolderEditor.svelte';
  import HostEditor from './HostEditor.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import Sidebar from './Sidebar.svelte';
  import TabBar from './TabBar.svelte';
  import Toasts from './Toasts.svelte';

  let { children }: { children: Snippet } = $props();

  $effect(() => () => reachability.clear());

  let mac = $state(false);
  let narrow = $state(false);

  onMount(() => {
    void api.appInfo().then((r) => {
      if (r.status !== 'ok') return;
      mac = r.data.platform === 'macos';
      // The local terminal starts with the app and keeps running (desktop only).
      if (['macos', 'windows', 'linux'].includes(r.data.platform))
        sessions.ensureLocal(m.local_terminal());
    });
    const mq = matchMedia('(max-width: 720px)');
    const sync = () => {
      if (mq.matches && !narrow) ui.sidebarOpen = false;
      narrow = mq.matches;
    };
    sync();
    mq.addEventListener('change', sync);
    return () => mq.removeEventListener('change', sync);
  });

  function typing(e: KeyboardEvent): boolean {
    const t = e.target;
    return (
      t instanceof HTMLElement &&
      (t.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName))
    );
  }

  /** The webview's own menu (Look Up, Inspect…) is only useful for text the user can act on. */
  function hideNativeMenu(e: MouseEvent) {
    const t = e.target;
    if (t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA)$/.test(t.tagName)))
      return;
    if (!window.getSelection()?.isCollapsed) return;
    e.preventDefault();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === '?' && !e.metaKey && !e.ctrlKey && !e.altKey && !typing(e)) {
      e.preventDefault();
      ui.hotkeysOpen = !ui.hotkeysOpen;
      return;
    }
    if (e.ctrlKey && !e.metaKey && !e.altKey && e.code === 'Backquote' && sessions.localTab) {
      e.preventDefault();
      toggleLocal();
      return;
    }
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      sessions.step(e.shiftKey ? -1 : 1);
      return;
    }
    if (!(isApple ? e.metaKey : e.ctrlKey) || e.altKey) return;
    const k = shortcutKey(e);
    if (e.shiftKey && k === 't') {
      const id = sessions.active?.hostId;
      if (id) {
        e.preventDefault();
        connectAnother(id);
      }
      return;
    }
    if (e.shiftKey && k !== 'tab') return;
    if (k === 'k') ui.paletteOpen = !ui.paletteOpen;
    else if (k === 'b') ui.sidebarOpen = !ui.sidebarOpen;
    else if (k === 'l') {
      if (vault.canLock) void vault.lock();
    } else if (k === ',') ui.settingsOpen = true;
    else if (k === 't') ui.paletteOpen = true;
    else if (k === 'w' && sessions.activeGroup) closeServer(sessions.activeGroup);
    else if (/^[1-9]$/.test(k)) {
      const g = sessions.visibleGroups[Number(k) - 1];
      if (g) sessions.select(g.current.id);
    } else return;
    e.preventDefault();
  }

  /** ⌃` : to the local terminal and back to where the user was. */
  let beforeLocal: string | null = null;
  function toggleLocal() {
    const local = sessions.localTab;
    if (!local) return;
    if (sessions.activeId !== local.id) {
      beforeLocal = sessions.activeId;
      sessions.select(local.id);
    } else if (beforeLocal && sessions.tabs.some((t) => t.id === beforeLocal))
      sessions.select(beforeLocal);
    else sessions.activeId = null;
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

<svelte:window onkeydown={onKeyDown} oncontextmenu={hideNativeMenu} />

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
<HostKeyDialog />
<CloseServerDialog />
<HotkeysDialog />
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
    position: relative;
    flex: 1;
    overflow: hidden;
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
