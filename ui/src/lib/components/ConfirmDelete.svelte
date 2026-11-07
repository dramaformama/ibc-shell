<script lang="ts">
  import { Dialog } from 'bits-ui';
  import type { FolderDeleteMode } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { folderSubtree, keyRef } from '$lib/util/tree-model';

  const keys = $derived(ui.deleteKeys ?? []);
  const refs = $derived(keys.map(keyRef));
  const hostIds = $derived(refs.filter((r) => r.kind === 'host').map((r) => r.id));
  const folderIds = $derived(refs.filter((r) => r.kind === 'folder').map((r) => r.id));
  /** Hosts that would vanish together with the chosen folders. */
  const inside = $derived.by(() => {
    const ids = new Set<string>();
    for (const f of folderIds) folderSubtree(tree.data, f).forEach((x) => ids.add(x));
    return tree.data.hosts.filter((h) => h.folder_id && ids.has(h.folder_id)).length;
  });
  let mode = $state<FolderDeleteMode>('MoveToParent');

  const title = $derived(
    keys.length === 1
      ? (tree.rows.find((r) => r.key === keys[0])?.label ?? '')
      : m.delete_n_items({ count: keys.length })
  );

  async function run() {
    const ids = hostIds;
    ui.deleteKeys = null;
    if (ids.length) await tree.deleteHosts(ids);
    for (const f of folderIds) await tree.deleteFolder(f, mode);
    tree.select([]);
    ui.toast(m.toast_deleted());
  }
</script>

<Dialog.Root open={ui.deleteKeys !== null} onOpenChange={(o) => !o && (ui.deleteKeys = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="confirm">
      <Dialog.Title>{m.delete_title({ name: title })}</Dialog.Title>
      <Dialog.Description>
        {#if folderIds.length}{m.delete_folder_body({
            hosts: inside
          })}{:else}{m.delete_host_body()}{/if}
      </Dialog.Description>
      {#if folderIds.length && inside > 0}
        <div class="seg" role="radiogroup" aria-label={m.delete_mode()}>
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'MoveToParent'}
            onclick={() => (mode = 'MoveToParent')}>{m.delete_keep_contents()}</button
          >
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'DeleteContents'}
            onclick={() => (mode = 'DeleteContents')}
            >{m.delete_with_contents({ hosts: inside })}</button
          >
        </div>
      {/if}
      <div class="actions">
        <Dialog.Close class="btn btn-ghost">{m.action_cancel()}</Dialog.Close>
        <button class="btn btn-primary danger-fill" onclick={run}>{m.action_delete()}</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.confirm) {
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(400px, calc(100vw - 32px));
    transform: translate(-50%, -50%);
    display: grid;
    gap: 12px;
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  :global(.confirm [data-dialog-title]) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  :global(.confirm [data-dialog-description]) {
    margin: 0;
    color: var(--muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .danger-fill {
    background: var(--danger);
    border-color: var(--danger);
  }
  .danger-fill:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 88%, black);
  }
  .seg {
    justify-self: start;
    display: inline-flex;
  }
</style>
