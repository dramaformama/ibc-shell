<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';

  interface Props {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }
  let { title, onclose, children, footer }: Props = $props();

  /** Focus the first form field instead of the close button. */
  function focusFirstField(e: Event) {
    const first = document.querySelector<HTMLElement>('.sheet .body input, .sheet .body button');
    if (first) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

<Dialog.Root open onOpenChange={(o) => !o && onclose()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sheet" onOpenAutoFocus={focusFirstField}>
      <header>
        <Dialog.Title>{title}</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label={m.action_close()}><X size={16} /></Dialog.Close>
      </header>
      <div class="body">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sheet) {
    position: fixed;
    z-index: 51;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(420px, 100vw);
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow);
    animation: slide 150ms var(--ease);
  }
  @keyframes slide {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 10px 20px;
  }
  header :global([data-dialog-title]) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .body {
    flex: 1;
    overflow: auto;
    padding: 4px 20px 20px;
    display: grid;
    gap: 14px;
    align-content: start;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px calc(12px + env(safe-area-inset-bottom));
    border-top: 1px solid var(--border);
  }
  @media (max-width: 640px) {
    :global(.sheet) {
      width: 100vw;
      border-left: 0;
    }
  }
</style>
