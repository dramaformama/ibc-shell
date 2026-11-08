<script lang="ts">
  import { api, type KnownHostSummary } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import { Trash2 } from '@lucide/svelte';

  const LIMIT = 200;

  let items = $state.raw<KnownHostSummary[]>([]);
  let loaded = $state(false);
  let query = $state('');
  let asking = $state<string | null>(null);
  let el = $state<HTMLElement>();

  void api.knownHostList().then((r) => {
    if (r.status === 'ok') items = r.data;
    loaded = true;
  });

  const matches = $derived(
    items.filter((k) => k.host_pattern.toLowerCase().includes(query.trim().toLowerCase()))
  );
  const shown = $derived(matches.slice(0, LIMIT));

  $effect(() => {
    if (el && loaded && ui.settingsFocus === 'known-hosts') {
      el.scrollIntoView({ block: 'start' });
      ui.settingsFocus = null;
    }
  });

  async function remove(id: string) {
    asking = null;
    const r = await api.knownHostDelete(id);
    if (r.status !== 'ok') return ui.toast(errorText(r.error));
    items = items.filter((k) => k.id !== id);
    ui.toast(m.toast_deleted());
  }
</script>

<section bind:this={el}>
  <h3>{m.settings_known()}</h3>
  <p class="note">{m.settings_known_note()}</p>
  {#if items.length > 8}
    <input
      class="input"
      type="search"
      bind:value={query}
      placeholder={m.settings_known_filter()}
      aria-label={m.settings_known_filter()}
    />
  {/if}
  {#if loaded && !items.length}
    <p class="note">{m.settings_known_empty()}</p>
  {/if}
  <ul class="rows">
    {#each shown as k (k.id)}
      <li>
        {#if asking === k.id}
          <span class="main">{m.known_confirm({ host: k.host_pattern })}</span>
          <span class="acts">
            <button type="button" class="btn btn-ghost" onclick={() => (asking = null)}
              >{m.action_cancel()}</button
            >
            <button
              type="button"
              class="btn btn-primary btn-danger-fill"
              onclick={() => void remove(k.id)}>{m.action_delete()}</button
            >
          </span>
        {:else}
          <span class="main">
            <span class="title">{k.host_pattern}</span>
            <span class="note mono">{k.key_type} · {k.fingerprint}</span>
          </span>
          <span class="acts">
            <button
              type="button"
              class="btn btn-ghost btn-danger icon"
              aria-label={m.action_delete()}
              title={m.action_delete()}
              onclick={() => (asking = k.id)}><Trash2 size={14} /></button
            >
          </span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if matches.length > LIMIT}
    <p class="note">{m.settings_known_capped({ count: LIMIT })}</p>
  {/if}
</section>

<style>
  section {
    scroll-margin-top: 8px;
  }
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
