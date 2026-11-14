<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { Activity, Power } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';

  /** Closing a live server: keep it connected for monitoring, or disconnect it. */
  const key = $derived(ui.closeAsk);
  const label = $derived(sessions.groups.find((g) => g.key === key)?.label ?? '');

  function keep() {
    if (key) sessions.hideGroup(key);
    ui.closeAsk = null;
  }
  function disconnect() {
    if (key) sessions.closeGroup(key);
    ui.closeAsk = null;
  }
</script>

<Dialog.Root open={!!key} onOpenChange={(o) => !o && (ui.closeAsk = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="closeask">
      <Dialog.Title>{m.close_ask_title({ name: label })}</Dialog.Title>
      <Dialog.Description>{m.close_ask_body()}</Dialog.Description>
      <div class="choices">
        <button class="choice" onclick={keep}>
          <span class="ic"><Activity size={16} /></span>
          <span class="tx"><b>{m.close_ask_keep()}</b><small>{m.close_ask_keep_hint()}</small></span
          >
        </button>
        <button class="choice danger" onclick={disconnect}>
          <span class="ic"><Power size={16} /></span>
          <span class="tx"
            ><b>{m.close_ask_disconnect()}</b><small>{m.close_ask_disconnect_hint()}</small></span
          >
        </button>
      </div>
      <div class="actions">
        <Dialog.Close class="btn btn-ghost">{m.action_cancel()}</Dialog.Close>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.closeask) {
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(420px, calc(100vw - 32px));
    transform: translate(-50%, -50%);
    animation: alert-in 340ms var(--spring);
    display: grid;
    gap: 12px;
    padding: 20px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-sheet);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  :global(.closeask [data-dialog-title]) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  :global(.closeask [data-dialog-description]) {
    margin: 0;
    color: var(--muted);
  }
  .choices {
    display: grid;
    gap: 8px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 0;
    border-radius: var(--r-field);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      background-color 120ms var(--ease),
      transform 120ms var(--ease);
  }
  .choice:hover {
    background: var(--hover);
  }
  .choice:active {
    transform: scale(0.99);
  }
  .ic {
    display: grid;
    place-items: center;
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .danger .ic {
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .tx {
    display: grid;
    line-height: 1.3;
  }
  .tx small {
    color: var(--muted);
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
