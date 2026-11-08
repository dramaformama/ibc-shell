<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { shortcutGroups } from './shortcuts';

  /** "Keyboard shortcuts": every key the app understands, grouped by what it acts on. */
  const groups = $derived(ui.hotkeysOpen ? shortcutGroups() : []);
</script>

<Dialog.Root open={ui.hotkeysOpen} onOpenChange={(o) => (ui.hotkeysOpen = o)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="hotkeys-dlg">
      <header>
        <Dialog.Title>{m.keys_title()}</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label={m.action_close()}><X size={16} /></Dialog.Close>
      </header>
      <Dialog.Description class="hint">{m.keys_hint()}</Dialog.Description>
      <div class="grid">
        {#each groups as g (g.title)}
          <section>
            <h3>{g.title}</h3>
            <ul>
              {#each g.items as it (it.label)}
                <li>
                  <span class="lbl">{it.label}</span>
                  <span class="caps" aria-label={it.keys.join(' ')}>
                    {#each it.keys as k, i (i)}<kbd class:sep={k === '–'}>{k}</kbd>{/each}
                  </span>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.hotkeys-dlg) {
    container-type: inline-size;
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(680px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    transform: translate(-50%, -50%);
    padding: 22px 22px 24px;
    border: 1px solid var(--border);
    border-radius: var(--r-sheet);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  :global(.hotkeys-dlg [data-dialog-title]) {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  :global(.hotkeys-dlg .hint) {
    margin: 4px 0 18px;
    color: var(--muted);
    font-size: 13px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }
  @container (max-width: 520px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  section {
    padding: 14px 16px 8px;
    border-radius: var(--r-field);
    background: var(--surface-2);
  }
  h3 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-height: 34px;
    font-size: 13px;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .lbl {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .caps {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 4px;
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 24px;
    height: 24px;
    padding: 0 7px;
    border-radius: 7px;
    background: var(--surface);
    box-shadow:
      inset 0 0 0 1px var(--border),
      0 1px 0 var(--border);
    font: 600 12px/1 var(--font-ui);
    color: var(--text);
  }
  kbd.sep {
    min-width: 0;
    padding: 0;
    background: none;
    box-shadow: none;
    color: var(--muted);
  }
</style>
