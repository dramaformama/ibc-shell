<script lang="ts">
  import { ContextMenu } from 'bits-ui';
  import { ChevronRight } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { connectFolder, connectHost, hostsInFolder } from '$lib/stores/actions';
  import { settings } from '$lib/stores/settings.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { folderSubtree, siblings, type NodeKey, type Row } from '$lib/util/tree-model';

  /** Context row (null = empty area) and the keys the action applies to. */
  let { row, keys }: { row: Row | null; keys: NodeKey[] } = $props();

  const multi = $derived(keys.length > 1);
  const only = $derived(keys.length === 1 ? row : null);

  const targets = $derived.by(() => {
    const banned = new Set<string>();
    for (const k of keys)
      if (k.startsWith('f:')) folderSubtree(tree.data, k.slice(2)).forEach((id) => banned.add(id));
    return tree.rows
      .filter((r) => r.kind === 'folder' && !banned.has(r.id))
      .map((r) => ({ id: r.id, label: r.label, depth: r.depth }));
  });

  function moveTo(parent: string | null) {
    const skip = new Set(keys);
    const sibs = siblings(tree.data, parent, skip);
    void tree.move(keys, {
      parent,
      after: sibs[sibs.length - 1] ?? null,
      rowIndex: -1,
      zone: 'inside'
    });
  }
  function flag(code: string | null) {
    if (code) settings.pushRecentFlag(code);
    for (const k of keys) void tree.setFlag(k, code);
  }
  function edit() {
    if (!row) return;
    if (row.kind === 'host') ui.openHostEditor(row.id);
    else ui.openFolderEditor(row.id);
  }
</script>

{#snippet item(label: string, onSelect: () => void, danger = false)}
  <ContextMenu.Item class="menu-item {danger ? 'danger' : ''}" {onSelect}>{label}</ContextMenu.Item>
{/snippet}

{#if !row}
  {@render item(m.action_new_host(), () => ui.openHostEditor(null))}
  {@render item(m.action_new_folder(), () => ui.openFolderEditor(null))}
{:else}
  {#if only?.kind === 'host'}
    {@render item(m.menu_connect(), () => connectHost(only.id))}
    {@render item(m.menu_sftp(), () => connectHost(only.id, 'sftp'))}
    {@render item(m.menu_status(), () => connectHost(only.id, 'status'))}
    <div class="menu-sep"></div>
    {@render item(m.menu_duplicate(), () => void tree.duplicate(only.id))}
    {@render item(m.menu_edit(), edit)}
  {:else if only?.kind === 'folder'}
    {@const n = hostsInFolder(only.id).length}
    {#if n}
      {@render item(m.menu_connect_all({ count: n }), () => connectFolder(only.id))}
      <div class="menu-sep"></div>
    {/if}
    {@render item(m.menu_new_host_here(), () => ui.openHostEditor(only.id, only.id))}
    {@render item(m.menu_new_subfolder(), () => ui.openFolderEditor(null, only.id))}
    <div class="menu-sep"></div>
    {@render item(m.menu_edit_defaults(), edit)}
  {/if}
  {#if only}
    {@render item(m.menu_rename(), () => (ui.renameKey = only.key))}
  {/if}

  <ContextMenu.Sub>
    <ContextMenu.SubTrigger class="menu-item">
      <span class="grow">{m.menu_set_flag()}</span><ChevronRight size={14} />
    </ContextMenu.SubTrigger>
    <ContextMenu.SubContent class="menu" sideOffset={4}>
      {@render item(m.flag_none(), () => flag(null))}
      {#each settings.recentFlags as code (code)}
        <ContextMenu.Item class="menu-item" onSelect={() => flag(code)}>
          <Flag {code} size={14} /><span>{code.toUpperCase()}</span>
        </ContextMenu.Item>
      {/each}
      {#if only}{@render item(m.menu_more(), edit)}{/if}
    </ContextMenu.SubContent>
  </ContextMenu.Sub>

  <ContextMenu.Sub>
    <ContextMenu.SubTrigger class="menu-item">
      <span class="grow">{m.menu_move_to()}</span><ChevronRight size={14} />
    </ContextMenu.SubTrigger>
    <ContextMenu.SubContent class="menu" sideOffset={4}>
      {@render item(m.menu_top_level(), () => moveTo(null))}
      {#each targets as t (t.id)}
        <ContextMenu.Item
          class="menu-item"
          style="padding-left: {8 + t.depth * 12}px"
          onSelect={() => moveTo(t.id)}>{t.label}</ContextMenu.Item
        >
      {/each}
    </ContextMenu.SubContent>
  </ContextMenu.Sub>

  <div class="menu-sep"></div>
  {@render item(
    multi ? m.menu_delete_n({ count: keys.length }) : m.menu_delete(),
    () => (ui.deleteKeys = keys),
    true
  )}
{/if}

<style>
  :global(.grow) {
    flex: 1;
  }
</style>
