<script lang="ts">
  import { ChevronDown, Folder, Pencil, Play } from '@lucide/svelte';
  import type { Server } from '$lib/fleet/model';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { connectFolder, hostsInFolder } from '$lib/stores/actions';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { pluralForm } from './format';

  /**
   * Folders on the overview: one card each with its servers' state; a click narrows the list
   * below to that folder (subfolders included), the play button connects the whole folder.
   */
  let { servers, selected = $bindable(null) }: { servers: Server[]; selected?: string | null } =
    $props();

  const OPEN_KEY = 'ibc.overview.folders';
  let open = $state(true);
  try {
    open = localStorage.getItem(OPEN_KEY) !== '0';
  } catch {
    /* storage unavailable */
  }
  function toggle() {
    open = !open;
    try {
      localStorage.setItem(OPEN_KEY, open ? '1' : '0');
    } catch {
      /* storage unavailable */
    }
  }

  const byId = $derived(new Map(servers.map((s) => [s.id, s])));
  const folders = $derived(
    tree.data.folders
      .map((f) => {
        const ids = hostsInFolder(f.id);
        const list = ids.map((id) => byId.get(id)).filter((s) => !!s);
        return {
          f,
          depth: depthOf(f.parent_id),
          total: ids.length,
          online: list.filter((s) => s.availability === 'online').length,
          connected: list.filter((s) => s.phase === 'connected').length
        };
      })
      .sort((a, b) => a.depth - b.depth || a.f.name.localeCompare(b.f.name))
  );

  const servers_n = (count: number) => {
    const f = pluralForm(count, getLocale());
    return f === 'one'
      ? m.dash_srv_one({ count })
      : f === 'few'
        ? m.dash_srv_few({ count })
        : m.dash_srv_many({ count });
  };

  function depthOf(parent: string | null): number {
    let d = 0;
    let p = parent;
    while (p && d < 32) {
      d++;
      p = tree.data.folders.find((x) => x.id === p)?.parent_id ?? null;
    }
    return d;
  }
</script>

{#if folders.length}
  <div class="sectionbar">
    <button class="fold" aria-expanded={open} onclick={toggle}>
      <ChevronDown size={15} class="chev" />
      <h2>{m.dash_folders()} <span class="tnum">{folders.length}</span></h2>
    </button>
  </div>
  {#if open}
    <div class="folders" role="group" aria-label={m.dash_folders()}>
      {#each folders as x (x.f.id)}
        <div class="folder" class:on={selected === x.f.id}>
          <button
            class="main"
            aria-pressed={selected === x.f.id}
            title={m.dash_folder_filter()}
            onclick={() => (selected = selected === x.f.id ? null : x.f.id)}
          >
            <span class="badge">
              {#if x.f.country_code}<Flag code={x.f.country_code} size={22} />{:else}<Folder
                  size={18}
                />{/if}
            </span>
            <span class="text">
              <span class="name">{x.f.name}</span>
              <span class="meta tnum">
                <span>{servers_n(x.total)}</span>
                {#if x.online}<span class="pip" title={m.dash_folder_online()}
                    ><i class="ok"></i>{x.online}</span
                  >{/if}
                {#if x.connected}<span class="pip" title={m.dash_folder_connected()}
                    ><i class="live"></i>{x.connected}</span
                  >{/if}
              </span>
            </span>
          </button>
          <span class="acts">
            <button
              class="icon-btn"
              title={m.folder_connect_all()}
              aria-label={m.folder_connect_all()}
              disabled={!x.total}
              onclick={() => connectFolder(x.f.id)}><Play size={14} /></button
            >
            <button
              class="icon-btn"
              title={m.dash_folder_edit()}
              aria-label={m.dash_folder_edit()}
              onclick={() => ui.openFolderEditor(x.f.id)}><Pencil size={13} /></button
            >
          </span>
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .fold {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: -4px;
    padding: 2px 4px;
    border: 0;
    border-radius: var(--r-inner);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .fold:hover {
    background: var(--hover);
  }
  .fold :global(.chev) {
    color: var(--muted);
    transition: rotate 150ms var(--ease);
  }
  .fold[aria-expanded='false'] :global(.chev) {
    rotate: -90deg;
  }
  .folders {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--folder-min, 230px), 1fr));
    gap: var(--tile-gap);
    margin-bottom: 28px;
  }
  .folder {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--card-shadow);
    transition: box-shadow 120ms var(--ease);
  }
  .folder.on {
    box-shadow:
      var(--card-shadow),
      0 0 0 1.5px var(--accent);
  }
  .main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: var(--folder-pad, 12px 4px 12px 14px);
    border: 0;
    border-radius: inherit;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .badge {
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 11px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .text {
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12px;
    line-height: 16px;
    white-space: nowrap;
  }
  .pip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .pip i {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .meta .ok {
    background: var(--ok);
  }
  .meta .live {
    background: var(--accent);
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-right: 8px;
    opacity: 0;
    transition: opacity 120ms var(--ease);
  }
  .folder:hover .acts,
  .acts:focus-within,
  .folder.on .acts {
    opacity: 1;
  }
  @media (hover: none) {
    .acts {
      opacity: 1;
    }
  }
  :global([data-density='compact']) .folders {
    --folder-min: 190px;
    --folder-pad: 6px 4px 6px 10px;
    margin-bottom: 16px;
  }
  :global([data-density='compact']) .badge {
    width: 28px;
    height: 28px;
    border-radius: 9px;
  }
  :global([data-density='compact']) .main {
    gap: 9px;
  }
</style>
