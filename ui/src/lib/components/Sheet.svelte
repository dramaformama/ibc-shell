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
    top: 10px;
    right: 10px;
    bottom: 10px;
    width: min(440px, calc(100vw - 20px));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 0.5px solid var(--border);
    border-radius: var(--r-sheet);
    background: var(--bg);
    box-shadow: var(--shadow);
    animation: slide 420ms var(--spring);
  }
  @keyframes slide {
    from {
      transform: translateX(16px) scale(0.99);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 56px;
    padding: 12px 14px 12px 22px;
    border-bottom: 0.5px solid var(--sep);
  }
  header :global([data-dialog-title]) {
    margin: 0;
    font-size: var(--fs-head);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .body {
    flex: 1;
    overflow: auto;
    padding: 20px 22px 24px;
    display: grid;
    gap: 14px;
    align-content: start;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px calc(12px + env(safe-area-inset-bottom));
    border-top: 0.5px solid var(--sep);
  }
  /* Destructive action sits alone at the start; Cancel / Save stay at the end. */
  footer :global(.btn-delete) {
    margin-inline-end: auto;
    gap: 6px;
  }
  @media (max-width: 640px) {
    :global(.sheet) {
      inset: auto 0 0 0;
      width: 100vw;
      max-height: 92vh;
      border-radius: var(--r-sheet) var(--r-sheet) 0 0;
    }
  }
</style>
