<script lang="ts">
  import { api, type SshImportEntry, type SshImportPreview } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import { SvelteSet } from 'svelte/reactivity';
  import Sheet from './Sheet.svelte';

  /** Preview of ~/.ssh/config with a checkbox per host; new hosts are pre-selected. */
  let preview = $state<SshImportPreview | null>(null);
  const picked = new SvelteSet<string>();
  let folder = $state('SSH config');
  let busy = $state(false);

  void api.sshConfigPreview().then((r) => {
    if (r.status === 'error') {
      ui.toast(errorText(r.error), 'error');
      return close();
    }
    preview = r.data;
    for (const e of r.data.entries) if (!e.duplicate) picked.add(e.alias);
  });

  const entries = $derived(preview?.entries ?? []);
  const allOn = $derived(entries.length > 0 && entries.every((e) => picked.has(e.alias)));

  function close() {
    ui.sshImportOpen = false;
  }

  function toggleAll() {
    if (allOn) picked.clear();
    else for (const e of entries) picked.add(e.alias);
  }

  function address(e: SshImportEntry) {
    return `${e.user ? `${e.user}@` : ''}${e.hostname}${e.port && e.port !== 22 ? `:${e.port}` : ''}`;
  }

  function keyNote(e: SshImportEntry): { text: string; warn: boolean } | null {
    const path = e.key_path ?? '';
    switch (e.key) {
      case 'ok':
        return { text: m.imp_key_ok({ path }), warn: false };
      case 'encrypted':
        return { text: m.imp_key_encrypted(), warn: true };
      case 'missing':
        return { text: m.imp_key_missing({ path }), warn: true };
      case 'invalid':
        return { text: m.imp_key_invalid({ path }), warn: true };
      default:
        return null;
    }
  }

  async function run() {
    busy = true;
    const r = await api.sshConfigImport([...picked], folder.trim());
    busy = false;
    if (r.status === 'error') return ui.toast(errorText(r.error), 'error');
    await tree.load();
    const s = r.data;
    ui.toast(m.imp_done({ hosts: s.hosts, keys: s.keys, forwards: s.forwards }));
    if (s.skipped_keys) ui.toast(m.imp_done_skipped({ count: s.skipped_keys }), 'info');
    close();
  }
</script>

<Sheet title={m.imp_title()} onclose={close}>
  {#if !preview}
    <p class="note">…</p>
  {:else if !preview.found}
    <p class="note">{m.imp_not_found()}</p>
  {:else if entries.length === 0}
    <p class="note">{m.imp_empty()}</p>
  {:else}
    <div class="fld">
      <label for="imp-folder">{m.imp_folder()}</label>
      <input id="imp-folder" class="input" bind:value={folder} />
    </div>
    <label class="all"
      ><input type="checkbox" checked={allOn} onchange={toggleAll} />{m.imp_all()}</label
    >
    <ul>
      {#each entries as e (e.alias)}
        {@const key = keyNote(e)}
        <li>
          <label>
            <input
              type="checkbox"
              checked={picked.has(e.alias)}
              onchange={() => (picked.has(e.alias) ? picked.delete(e.alias) : picked.add(e.alias))}
            />
            <span class="what">
              <span class="name">{e.alias}</span>
              <span class="addr mono">{address(e)}</span>
              <span class="tags">
                {#if e.duplicate}<span class="tag">{m.imp_duplicate()}</span>{/if}
                {#if e.proxy_jump}<span class="tag">{m.imp_via({ hop: e.proxy_jump })}</span>{/if}
                {#if e.forwards}<span class="tag">{m.imp_forwards({ count: e.forwards })}</span
                  >{/if}
                {#each e.unsupported as what (what)}<span class="tag warn"
                    >{m.imp_unsupported({ what })}</span
                  >{/each}
              </span>
              {#if key}<span class="key" class:warn={key.warn}>{key.text}</span>{/if}
            </span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
  {#snippet footer()}
    <button type="button" class="btn" onclick={close}>{m.action_cancel()}</button>
    {#if preview?.found && entries.length}
      <button
        type="button"
        class="btn btn-primary"
        disabled={busy || picked.size === 0}
        onclick={run}>{m.imp_run({ count: picked.size })}</button
      >
    {/if}
  {/snippet}
</Sheet>

<style>
  .all {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li + li {
    border-top: 0.5px solid var(--border);
  }
  li label {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 9px 0;
    cursor: pointer;
  }
  li input {
    margin-top: 3px;
  }
  .what {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  .addr {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tags:empty {
    display: none;
  }
  .tag {
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--muted);
    font-size: 11px;
  }
  .key {
    font-size: 12px;
    color: var(--muted);
  }
  .warn {
    color: var(--warn);
  }
</style>
