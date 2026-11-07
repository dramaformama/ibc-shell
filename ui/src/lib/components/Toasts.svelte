<script lang="ts">
  import { X } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <div class="toast" class:error={t.tone === 'error'}>
      <span>{t.text}</span>
      <button class="icon-btn" aria-label={m.action_close()} onclick={() => ui.dismiss(t.id)}
        ><X size={14} /></button
      >
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    z-index: 80;
    right: 16px;
    bottom: calc(16px + env(safe-area-inset-bottom));
    display: grid;
    gap: 8px;
    max-width: min(360px, calc(100vw - 32px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 8px 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--shadow);
    animation: rise 150ms var(--ease);
  }
  .toast.error {
    border-color: color-mix(in oklab, var(--danger) 50%, var(--border));
  }
  .toast span {
    flex: 1;
    overflow-wrap: anywhere;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
