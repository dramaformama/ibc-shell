<script lang="ts">
  import { api, type IdentityDto } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import { Copy, Trash2 } from '@lucide/svelte';

  let items = $state.raw<IdentityDto[]>([]);
  let loaded = $state(false);
  let asking = $state<string | null>(null);

  void api.identityList().then((r) => {
    if (r.status === 'ok') items = r.data;
    loaded = true;
  });

  const kinds = {
    password: m.key_kind_password,
    key: m.key_kind_key,
    key_password: m.key_kind_key_password,
    agent: m.key_kind_agent
  };

  const uses = (id: string) =>
    tree.data.hosts.filter((h) => h.identity_id === id).length +
    tree.data.folders.filter((f) => f.defaults.identity_id === id).length;

  async function remove(id: string) {
    asking = null;
    const r = await api.identityDelete(id);
    if (r.status !== 'ok') return ui.toast(errorText(r.error));
    items = items.filter((i) => i.id !== id);
    await tree.load();
    ui.toast(m.toast_deleted());
  }

  function copy(text: string) {
    void navigator.clipboard.writeText(text).then(() => ui.toast(m.toast_copied()));
  }
</script>

<section>
  <h3>{m.settings_keys()}</h3>
  <p class="note">{m.settings_keys_note()}</p>
  {#if loaded && !items.length}
    <p class="note">{m.settings_keys_empty()}</p>
  {/if}
  <ul class="rows">
    {#each items as i (i.id)}
      {@const n = uses(i.id)}
      <li>
        {#if asking === i.id}
          <span class="main">
            {n ? m.key_confirm_used({ name: i.label, count: n }) : m.key_confirm({ name: i.label })}
          </span>
          <span class="acts">
            <button type="button" class="btn btn-ghost" onclick={() => (asking = null)}
              >{m.action_cancel()}</button
            >
            <button
              type="button"
              class="btn btn-primary btn-danger-fill"
              onclick={() => void remove(i.id)}>{m.action_delete()}</button
            >
          </span>
        {:else}
          <span class="main">
            <span class="title">{i.label}</span>
            <span class="note">
              {kinds[i.kind]()}{i.key_algo ? ` · ${i.key_algo}` : ''}{i.username
                ? ` · ${i.username}`
                : ''} · {m.key_used({ count: n })}
            </span>
          </span>
          <span class="acts">
            {#if i.public_key}
              {@const pub = i.public_key}
              <button
                type="button"
                class="btn btn-ghost icon"
                aria-label={m.key_copy_public()}
                title={m.key_copy_public()}
                onclick={() => copy(pub)}><Copy size={14} /></button
              >
            {/if}
            <button
              type="button"
              class="btn btn-ghost btn-danger icon"
              aria-label={m.action_delete()}
              title={m.action_delete()}
              onclick={() => (asking = i.id)}><Trash2 size={14} /></button
            >
          </span>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  .rows {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 0;
  }
  .main {
    display: grid;
    flex: 1 1 160px;
    min-width: 0;
    gap: 2px;
    overflow-wrap: anywhere;
  }
  .title {
    font-weight: 500;
  }
  .acts {
    display: flex;
    gap: 4px;
    margin-inline-start: auto;
  }
  .icon {
    width: var(--control-h);
    padding: 0;
    justify-content: center;
  }
</style>
