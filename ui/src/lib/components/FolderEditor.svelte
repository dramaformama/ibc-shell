<script lang="ts">
  import { Trash2 } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { fkey, inheritedFlag } from '$lib/util/tree-model';
  import FlagPicker from './FlagPicker.svelte';
  import Sheet from './Sheet.svelte';

  let { id, parentId }: { id: string | null; parentId: string | null } = $props();

  // The editor is re-created per target ({#key} in Shell), so initial props are intentional.
  /* svelte-ignore state_referenced_locally */
  const folder = id ? tree.folder(id) : undefined;
  /* svelte-ignore state_referenced_locally */
  const parent = folder ? folder.parent_id : parentId;

  let name = $state(folder?.name ?? '');
  let flag = $state<string | null>(folder?.country_code ?? null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const inheritedF = $derived(inheritedFlag(tree.data, parent));

  async function save() {
    error = null;
    if (!name.trim()) return (error = m.err_name_required());
    busy = true;
    const ok = folder
      ? await tree.updateFolder(folder.id, {
          ...(name.trim() !== folder.name ? { name: name.trim() } : {}),
          ...(flag !== folder.country_code ? { country_code: flag } : {})
        })
      : (await tree.createFolder({
          parent_id: parent,
          name: name.trim(),
          country_code: flag,
          defaults: {
            username: null,
            port: null,
            identity_id: null,
            jump_host_id: null,
            startup_cmd: null,
            keepalive_s: null,
            term_theme: null
          }
        })) !== null;
    busy = false;
    if (ok) {
      ui.closeEditor();
      ui.toast(m.toast_saved());
    }
  }
</script>

<Sheet
  title={folder ? m.editor_folder_edit() : m.editor_folder_new()}
  onclose={() => ui.closeEditor()}
>
  <form id="folder-form" class="form" onsubmit={(e) => (e.preventDefault(), void save())}>
    <section>
      <div class="fld">
        <label for="f-name">{m.field_name()}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="f-name" class="input" bind:value={name} autocomplete="off" autofocus />
      </div>
      <div class="fld">
        <span class="lbl">{m.field_flag()}</span>
        <FlagPicker value={flag} inherited={inheritedF} onchange={(c) => (flag = c)} />
      </div>
    </section>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </form>
  {#snippet footer()}
    {#if folder}
      <button
        type="button"
        class="btn btn-ghost btn-danger btn-delete"
        disabled={busy}
        onclick={() => ui.deleteFromEditor(fkey(folder.id))}
        ><Trash2 size={14} />{m.action_delete()}</button
      >
    {/if}
    <button type="button" class="btn btn-ghost" onclick={() => ui.closeEditor()}
      >{m.action_cancel()}</button
    >
    <button type="submit" form="folder-form" class="btn btn-primary" disabled={busy}
      >{m.action_save()}</button
    >
  {/snippet}
</Sheet>
