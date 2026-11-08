<script lang="ts">
  import { Braces, Trash2 } from '@lucide/svelte';
  import Sheet from '$lib/components/Sheet.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { parseVars } from './model';
  import { snippets } from './snippets.svelte';

  const NEW = '__new__';
  const start = snippets.editing;
  const editing = start?.snippet ?? null;
  let name = $state(editing?.name ?? '');
  let body = $state(editing?.body ?? '');
  let group = $state<string>((editing ? snippets.layout.of[editing.id] : start?.group) ?? '');
  let newGroup = $state('');
  let busy = $state(false);
  const vars = $derived(parseVars(body));
  const valid = $derived(!!name.trim() && !!body.trim() && (group !== NEW || !!newGroup.trim()));

  const close = () => (snippets.editing = null);

  async function submit(e: Event) {
    e.preventDefault();
    if (!valid || busy) return;
    busy = true;
    const g = group === NEW ? snippets.addGroup(newGroup.trim()) : group || null;
    if (await snippets.save(name.trim(), body, g, editing?.id)) close();
    busy = false;
  }

  async function remove() {
    if (!editing) return;
    await snippets.remove(editing.id);
    close();
  }
</script>

<Sheet title={editing ? m.snip_edit() : m.snip_new()} onclose={close}>
  <form id="snippet-form" class="form" onsubmit={submit}>
    <section>
      <div class="fld">
        <label for="snip-name">{m.snip_name()}</label>
        <input
          id="snip-name"
          class="input"
          bind:value={name}
          placeholder={m.snip_name_hint()}
          autocomplete="off"
        />
      </div>
      <div class="fld">
        <label for="snip-group">{m.snip_folder()}</label>
        <select id="snip-group" class="input" bind:value={group}>
          <option value="">{m.snip_no_folder()}</option>
          {#each snippets.layout.groups as g (g.id)}<option value={g.id}>{g.name}</option>{/each}
          <option value={NEW}>{m.snip_new_folder()}…</option>
        </select>
        {#if group === NEW}
          <input
            class="input"
            bind:value={newGroup}
            placeholder={m.snip_folder_name()}
            aria-label={m.snip_folder_name()}
          />
        {/if}
      </div>
    </section>
    <section>
      <div class="fld">
        <label for="snip-body">{m.snip_command()}</label>
        <textarea
          id="snip-body"
          class="input mono body"
          rows="7"
          spellcheck="false"
          autocapitalize="off"
          bind:value={body}
          placeholder={'docker logs -f --tail {{lines:100}} {{container}}'}></textarea>
      </div>
      <p class="hint">
        <Braces size={13} />{m.snip_vars_hint({ a: '{{host}}', b: '{{port:22}}' })}
      </p>
      {#if vars.length}
        <div class="chips" aria-label={m.snip_vars()}>
          {#each vars as v (v.name)}
            <span class="var mono">{v.name}{v.fallback ? ` = ${v.fallback}` : ''}</span>
          {/each}
        </div>
      {/if}
    </section>
  </form>
  {#snippet footer()}
    {#if editing}
      <button class="btn btn-ghost btn-danger btn-delete" type="button" onclick={remove}
        ><Trash2 size={14} />{m.action_delete()}</button
      >
    {/if}
    <button class="btn btn-ghost" type="button" onclick={close}>{m.action_cancel()}</button>
    <button class="btn btn-primary" type="submit" form="snippet-form" disabled={!valid || busy}
      >{m.action_save()}</button
    >
  {/snippet}
</Sheet>

<style>
  .body {
    min-height: 140px;
    line-height: 1.5;
  }
  .hint {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
  .hint :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .var {
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
