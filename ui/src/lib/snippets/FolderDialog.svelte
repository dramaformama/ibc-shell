<script lang="ts">
  import { Dialog } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { snippets } from './snippets.svelte';

  /** Asks for a snippet folder's name (create / rename) or confirms its deletion. */
  const MAX = 60;
  const naming = $derived(snippets.folderDialog);
  const doomedId = $derived(snippets.folderDelete);
  const doomed = $derived(snippets.layout.groups.find((g) => g.id === doomedId) ?? null);
  const inside = $derived(
    doomedId ? snippets.items.filter((s) => snippets.layout.of[s.id] === doomedId).length : 0
  );

  let name = $state('');
  $effect(() => {
    if (naming)
      name = naming.id ? (snippets.layout.groups.find((g) => g.id === naming.id)?.name ?? '') : '';
  });
  const trimmed = $derived(name.trim());
  const taken = $derived(
    !!trimmed &&
      snippets.layout.groups.some(
        (g) => g.id !== naming?.id && g.name.toLocaleLowerCase() === trimmed.toLocaleLowerCase()
      )
  );
  const valid = $derived(!!trimmed && !taken);

  function submit(e: Event) {
    e.preventDefault();
    if (!valid || !naming) return;
    if (naming.id) snippets.renameGroup(naming.id, trimmed);
    else snippets.addGroup(trimmed);
    snippets.folderDialog = null;
  }
  function remove() {
    if (doomedId) snippets.removeGroup(doomedId);
    snippets.folderDelete = null;
  }
</script>

<Dialog.Root open={!!naming} onOpenChange={(o) => !o && (snippets.folderDialog = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="fdlg">
      <Dialog.Title>{naming?.id ? m.snip_folder_rename() : m.snip_new_folder()}</Dialog.Title>
      <form onsubmit={submit}>
        <input
          class="input"
          bind:value={name}
          maxlength={MAX}
          placeholder={m.snip_folder_hint()}
          aria-label={m.snip_folder_name()}
          aria-invalid={taken}
          autocomplete="off"
        />
        {#if taken}<p class="err" role="alert">{m.snip_folder_exists()}</p>{/if}
        <div class="actions">
          <Dialog.Close type="button" class="btn btn-ghost">{m.action_cancel()}</Dialog.Close>
          <button class="btn btn-primary" type="submit" disabled={!valid}
            >{naming?.id ? m.action_save() : m.action_create()}</button
          >
        </div>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<Dialog.Root open={!!doomed} onOpenChange={(o) => !o && (snippets.folderDelete = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="fdlg">
      <Dialog.Title>{m.snip_folder_delete_title({ name: doomed?.name ?? '' })}</Dialog.Title>
      <Dialog.Description>
        {inside ? m.snip_folder_delete_body({ count: inside }) : m.snip_folder_delete_empty()}
      </Dialog.Description>
      <div class="actions">
        <Dialog.Close class="btn btn-ghost">{m.action_cancel()}</Dialog.Close>
        <button class="btn btn-primary danger-fill" onclick={remove}>{m.action_delete()}</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.fdlg) {
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(380px, calc(100vw - 32px));
    transform: translate(-50%, -50%);
    display: grid;
    gap: 12px;
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  :global(.fdlg [data-dialog-title]) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  :global(.fdlg [data-dialog-description]) {
    margin: 0;
    color: var(--muted);
  }
  form {
    display: grid;
    gap: 10px;
  }
  .err {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
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
</style>
