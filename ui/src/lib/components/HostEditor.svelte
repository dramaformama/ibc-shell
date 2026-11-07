<script lang="ts">
  import { api, type Tag } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { emptyDefaults, formFrom, toPatch, toValues } from '$lib/util/defaults-form';
  import { hkey, inheritedFlag, siblings } from '$lib/util/tree-model';
  import DefaultsFields from './DefaultsFields.svelte';
  import FlagPicker from './FlagPicker.svelte';
  import Sheet from './Sheet.svelte';

  let { id, folderId }: { id: string | null; folderId: string | null } = $props();

  // The editor is re-created per target ({#key} in Shell), so initial props are intentional.
  /* svelte-ignore state_referenced_locally */
  const host = id ? tree.host(id) : undefined;

  let label = $state(host?.label ?? '');
  let hostname = $state(host?.hostname ?? '');
  let flag = $state<string | null>(host?.country_code ?? null);
  /* svelte-ignore state_referenced_locally */
  let folder = $state<string | null>(host ? host.folder_id : folderId);
  let notes = $state(host?.notes ?? '');
  let tagIds = $state.raw<string[]>(host?.tag_ids ?? []);
  let defaults = $state(host ? formFrom(host) : emptyDefaults());
  let tags = $state.raw<Tag[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);

  void api.tagList().then((r) => r.status === 'ok' && (tags = r.data));

  const inheritedF = $derived(inheritedFlag(tree.data, folder));
  const folderOptions = $derived(tree.data.folders.map((f) => ({ id: f.id, name: f.name })));

  function validate(): string | null {
    if (!label.trim()) return m.err_label_required();
    if (!hostname.trim()) return m.err_hostname_required();
    const p = defaults.port.trim();
    if (p && !(/^\d+$/.test(p) && +p >= 1 && +p <= 65535)) return m.err_port_range();
    return null;
  }

  async function save() {
    error = validate();
    if (error) return;
    busy = true;
    let ok: boolean;
    if (host) {
      const patch = {
        ...(label.trim() !== host.label ? { label: label.trim() } : {}),
        ...(hostname.trim() !== host.hostname ? { hostname: hostname.trim() } : {}),
        ...(flag !== host.country_code ? { country_code: flag } : {}),
        ...(notes !== (host.notes ?? '') ? { notes: notes || null } : {}),
        ...(tagIds.join() !== host.tag_ids.join() ? { tag_ids: tagIds } : {}),
        ...toPatch(formFrom(host), defaults)
      };
      ok = await tree.updateHost(host.id, patch);
      if (ok && folder !== host.folder_id) {
        const sibs = siblings(tree.data, folder, new Set([hkey(host.id)]));
        ok = await tree.move([hkey(host.id)], {
          parent: folder,
          after: sibs[sibs.length - 1] ?? null,
          rowIndex: -1,
          zone: 'inside'
        });
      }
    } else {
      ok =
        (await tree.createHost({
          folder_id: folder,
          label: label.trim(),
          hostname: hostname.trim(),
          country_code: flag,
          notes: notes || null,
          tag_ids: tagIds,
          ...toValues(defaults)
        })) !== null;
    }
    busy = false;
    if (ok) {
      ui.closeEditor();
      ui.toast(m.toast_saved());
    }
  }
  const toggleTag = (t: string) =>
    (tagIds = tagIds.includes(t) ? tagIds.filter((x) => x !== t) : [...tagIds, t]);
</script>

<Sheet title={host ? m.editor_host_edit() : m.editor_host_new()} onclose={() => ui.closeEditor()}>
  <form id="host-form" class="form" onsubmit={(e) => (e.preventDefault(), void save())}>
    <section>
      <h3>{m.section_general()}</h3>
      <div class="fld">
        <label for="h-label">{m.field_label()}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="h-label" class="input" bind:value={label} autocomplete="off" autofocus />
      </div>
      <div class="fld">
        <label for="h-host">{m.field_hostname()}</label>
        <input
          id="h-host"
          class="input mono"
          bind:value={hostname}
          autocomplete="off"
          spellcheck="false"
          placeholder="203.0.113.7"
        />
      </div>
      <div class="fld">
        <span class="lbl">{m.field_flag()}</span>
        <FlagPicker value={flag} inherited={inheritedF} onchange={(c) => (flag = c)} />
      </div>
      <div class="fld">
        <label for="h-folder">{m.field_folder()}</label>
        <select
          id="h-folder"
          class="input"
          value={folder ?? ''}
          onchange={(e) => (folder = e.currentTarget.value || null)}
        >
          <option value="">{m.menu_top_level()}</option>
          {#each folderOptions as f (f.id)}<option value={f.id}>{f.name}</option>{/each}
        </select>
      </div>
      {#if tags.length}
        <div class="fld">
          <span class="lbl">{m.field_tags()}</span>
          <div class="chips">
            {#each tags as t (t.id)}
              <button
                type="button"
                class="chip"
                aria-pressed={tagIds.includes(t.id)}
                onclick={() => toggleTag(t.id)}>{t.name}</button
              >
            {/each}
          </div>
        </div>
      {/if}
    </section>

    <DefaultsFields bind:form={defaults} inheritFrom={folder} selfHostId={host?.id ?? null} />

    <section>
      <h3>{m.section_notes()}</h3>
      <textarea class="input" rows="3" bind:value={notes} aria-label={m.section_notes()}></textarea>
    </section>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </form>
  {#snippet footer()}
    <button type="button" class="btn btn-ghost" onclick={() => ui.closeEditor()}
      >{m.action_cancel()}</button
    >
    <button type="submit" form="host-form" class="btn btn-primary" disabled={busy}
      >{m.action_save()}</button
    >
  {/snippet}
</Sheet>
