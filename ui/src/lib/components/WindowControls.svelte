<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Copy, Minus, Square, X } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';

  const win = isTauri() ? getCurrentWindow() : null;
  let maximized = $state(false);

  onMount(() => {
    if (!win) return;
    let live = true;
    async function sync() {
      const next = await win?.isMaximized().catch(() => undefined);
      if (live && next !== undefined) maximized = next;
    }
    const off = win.onResized(sync).catch(() => undefined);
    void sync();
    return () => {
      live = false;
      void off.then((unlisten) => unlisten?.()).catch(() => {});
    };
  });

  function run(action: () => Promise<void>) {
    void action().catch(() => ui.toast(m.err_generic(), 'error'));
  }
</script>

{#if win}
  <div class="window-controls" role="group" aria-label={m.window_controls()}>
    <button
      type="button"
      class="icon-btn"
      aria-label={m.window_minimize()}
      title={m.window_minimize()}
      onclick={() => run(() => win.minimize())}><Minus size={15} /></button
    >
    <button
      type="button"
      class="icon-btn"
      aria-label={maximized ? m.window_restore() : m.window_maximize()}
      title={maximized ? m.window_restore() : m.window_maximize()}
      onclick={() => run(() => win.toggleMaximize())}
    >
      {#if maximized}<Copy size={13} />{:else}<Square size={12} />{/if}
    </button>
    <button
      type="button"
      class="icon-btn close-window"
      aria-label={m.window_close()}
      title={m.window_close()}
      onclick={() => run(() => win.close())}><X size={15} /></button
    >
  </div>
{/if}

<style>
  .window-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .close-window:hover {
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 12%, transparent);
  }
</style>
