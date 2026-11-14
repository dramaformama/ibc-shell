<script lang="ts">
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { ContextMenu } from 'bits-ui';
  import { ArrowDown, ArrowUp, File, Folder, Link2 } from '@lucide/svelte';
  import type { Entry } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { dnd } from './dnd';
  import { formatDate, formatMode, formatSize, type Actions, type SortKey } from './model';
  import type { Pane } from './paneState.svelte';
  import { drag, sftpLayout, type ColKey } from './layout.svelte';
  import VList from './VList.svelte';

  let {
    pane,
    id,
    dropName,
    act
  }: { pane: Pane; id: 'left' | 'right'; dropName: string | null; act: Actions } = $props();

  let list = $state<ReturnType<typeof VList<Entry>>>();
  let scroller = $state<HTMLDivElement | null>(null);

  const cols: { key: SortKey; label: () => string; cls: string }[] = [
    { key: 'name', label: () => m.sftp_col_name(), cls: 'c-name' },
    { key: 'size', label: () => m.sftp_col_size(), cls: 'c-size' },
    { key: 'mtime', label: () => m.sftp_col_modified(), cls: 'c-mtime' },
    { key: 'mode', label: () => m.sftp_col_perms(), cls: 'c-mode' }
  ];

  /** Drags the left border of a fixed column: left widens it, right narrows it. */
  function startCol(e: PointerEvent, k: ColKey) {
    const w0 = sftpLayout.cols[k];
    drag(
      e,
      (dx) => sftpLayout.setCol(k, w0 - dx),
      () => sftpLayout.save()
    );
  }

  const sortLabel = (k: SortKey) =>
    pane.sort.key !== k ? 'none' : pane.sort.dir === 'asc' ? 'ascending' : 'descending';

  function onRowClick(e: MouseEvent, name: string) {
    pane.click(name, { shift: e.shiftKey, meta: e.metaKey || e.ctrlKey });
  }

  function move(delta: number, shift: boolean) {
    const order = pane.order;
    if (!order.length) return;
    const cur = pane.selection.anchor;
    const i = cur ? order.indexOf(cur) : -1;
    const next = Math.min(order.length - 1, Math.max(0, i === -1 ? 0 : i + delta));
    if (shift) {
      const anchor = pane.selection.anchor ?? order[next]!;
      pane.selection = {
        names: new Set(
          order.slice(
            Math.min(order.indexOf(anchor), next),
            Math.max(order.indexOf(anchor), next) + 1
          )
        ),
        anchor
      };
    } else {
      pane.click(order[next]!, {});
    }
    list?.reveal(next);
  }

  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('input, textarea')) return;
    const one = pane.selected.length === 1 ? pane.selected[0] : null;
    const mod = e.metaKey || e.ctrlKey;
    if (e.key === 'ArrowDown') move(1, e.shiftKey);
    else if (e.key === 'ArrowUp') move(-1, e.shiftKey);
    else if (e.key === 'Enter' && one) act.open(one);
    else if (
      e.code === 'Space' &&
      !mod &&
      !e.altKey &&
      !e.shiftKey &&
      !e.repeat &&
      one &&
      (one.kind === 'file' || one.kind === 'symlink') &&
      !(e.target as HTMLElement).closest('button, a')
    )
      act.open(one);
    else if (e.key === 'F2' && one) act.rename(one);
    else if ((e.key === 'Delete' || (e.key === 'Backspace' && mod)) && pane.selected.length)
      act.remove();
    else if (e.key === 'Backspace') void pane.up();
    else if (mod && e.key.toLowerCase() === 'a') pane.selectAll();
    else if (e.key === 'Escape') pane.clearSelection();
    else return;
    e.preventDefault();
  }

  function dragStart(e: DragEvent, entry: Entry) {
    if (!pane.selection.names.has(entry.name)) pane.click(entry.name, {});
    dnd.source = { pane: id, names: [...pane.selection.names] };
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'copy';
      e.dataTransfer.setData('text/plain', entry.name);
      const ghost = document.createElement('div');
      ghost.className = 'sf-ghost';
      ghost.textContent =
        pane.selection.names.size > 1
          ? m.sftp_drag_items({ count: pane.selection.names.size })
          : entry.name;
      document.body.append(ghost);
      e.dataTransfer.setDragImage(ghost, -8, -8);
      setTimeout(() => ghost.remove());
    }
  }
</script>

<div
  class="sf-table"
  role="grid"
  aria-label={m.sftp_files()}
  aria-multiselectable="true"
  tabindex="0"
  onkeydown={onKey}
>
  <div class="sf-head" role="row">
    {#each cols as c (c.key)}
      {#if c.key !== 'name'}
        {@const k = c.key as ColKey}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="sf-grip {c.cls}"
          title={m.sftp_resize_col()}
          onpointerdown={(e) => startCol(e, k)}
          ondblclick={() => sftpLayout.resetCol(k)}
        ></span>
      {/if}
      <button
        type="button"
        role="columnheader"
        class="sf-th {c.cls}"
        aria-sort={sortLabel(c.key)}
        onclick={() => pane.toggleSort(c.key)}
      >
        {c.label()}
        {#if pane.sort.key === c.key}
          {#if pane.sort.dir === 'asc'}<ArrowUp size={11} />{:else}<ArrowDown size={11} />{/if}
        {/if}
      </button>
    {/each}
  </div>

  <ContextMenu.Root>
    <ContextMenu.Trigger class="sf-body">
      <VList bind:this={list} bind:scroller items={pane.view} rowHeight={30} key={(e) => e.name}>
        {#snippet row(entry)}
          {@const sel = pane.selection.names.has(entry.name)}
          <div
            class="sf-row"
            class:sel
            class:drop={dropName === entry.name}
            role="row"
            tabindex="-1"
            aria-selected={sel}
            data-dir={entry.kind === 'dir' ? entry.name : undefined}
            draggable="true"
            onclick={(e) => onRowClick(e, entry.name)}
            ondblclick={() => act.open(entry)}
            oncontextmenu={() => pane.contextOn(entry.name)}
            ondragstart={(e) => dragStart(e, entry)}
            ondragend={() => (dnd.source = null)}
            onkeydown={null}
          >
            <span class="c-name" title={entry.name}>
              {#if entry.kind === 'dir'}<Folder size={15} class="ic ic-dir" />
              {:else if entry.kind === 'symlink'}<Link2 size={15} class="ic" />
              {:else}<File size={15} class="ic" />{/if}
              <span class="nm">{entry.name}</span>
            </span>
            <span class="c-size tnum">{entry.kind === 'dir' ? '' : formatSize(entry.size)}</span>
            <span class="c-mtime tnum">{formatDate(entry.mtime)}</span>
            <span class="c-mode mono">{formatMode(entry.mode, entry.kind)}</span>
          </div>
        {/snippet}
        {#snippet empty()}
          {#if pane.loading}
            <div class="sf-sk">
              <Skeleton rows={6} two={false} label={m.sftp_loading()} />
            </div>
          {:else}
            <p class="sf-empty">
              {#if pane.error}{m.sftp_load_error()}
              {:else if pane.filter}{m.sftp_no_matches()}
              {:else}{m.sftp_empty_dir()}{/if}
            </p>
          {/if}
        {/snippet}
      </VList>
    </ContextMenu.Trigger>
    <ContextMenu.Portal>
      <ContextMenu.Content class="menu">
        {@const one = pane.selected.length === 1 ? pane.selected[0] : null}
        {#if one}
          <ContextMenu.Item class="menu-item" onSelect={() => act.open(one)}
            >{one.kind === 'dir' ? m.sftp_menu_open() : m.sftp_preview()}</ContextMenu.Item
          >
        {/if}
        {#if pane.selected.length && act.canTransfer}
          <ContextMenu.Item class="menu-item" onSelect={act.transfer}
            >{act.transferLabel}</ContextMenu.Item
          >
        {/if}
        {#if one}
          <ContextMenu.Item class="menu-item" onSelect={() => act.rename(one)}
            >{m.menu_rename()}<span class="kbd push">F2</span></ContextMenu.Item
          >
        {/if}
        <ContextMenu.Item class="menu-item" onSelect={act.newFolder}
          >{m.action_new_folder()}</ContextMenu.Item
        >
        {#if one}
          <ContextMenu.Item class="menu-item" onSelect={() => act.copyPath(one)}
            >{m.sftp_copy_path()}</ContextMenu.Item
          >
          {#if act.canPerms}
            <ContextMenu.Item class="menu-item" onSelect={() => act.perms(one)}
              >{m.sftp_permissions()}</ContextMenu.Item
            >
          {/if}
        {/if}
        {#if pane.selected.length}
          <div class="menu-sep"></div>
          <ContextMenu.Item class="menu-item danger" onSelect={act.remove}
            >{pane.selected.length > 1
              ? m.menu_delete_n({ count: pane.selected.length })
              : m.menu_delete()}</ContextMenu.Item
          >
        {/if}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
</div>
