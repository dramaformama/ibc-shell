<script lang="ts">
  import { ChevronRight, Folder as FolderIcon, FolderOpen, SquareTerminal } from '@lucide/svelte';
  import { connectHost } from '$lib/stores/actions';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import type { Row } from '$lib/util/tree-model';

  interface Props {
    row: Row;
    oncommit: (value: string) => void;
    oncancel: () => void;
  }
  let { row, oncommit, oncancel }: Props = $props();

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

{#if row.kind === 'folder'}
  <button
    type="button"
    class="chev"
    tabindex="-1"
    aria-label={row.expanded ? m.tree_collapse() : m.tree_expand()}
    onclick={(e) => {
      e.stopPropagation();
      void tree.setExpanded(row.id, !row.expanded);
    }}
  >
    <ChevronRight size={14} class={row.expanded ? 'open' : ''} />
  </button>
  {#if row.flag}<Flag code={row.flag} size={16} />{:else}<FolderIcon
      size={16}
      class="folder-ic"
    />{/if}
{:else}
  <span class="chev"></span>
  <Flag code={row.flag} size={16} />
{/if}

{#if ui.renameKey === row.key}
  <input
    class="rename"
    value={row.label}
    use:focusSelect
    onclick={(e) => e.stopPropagation()}
    onpointerdown={(e) => e.stopPropagation()}
    onblur={(e) => ui.renameKey === row.key && oncommit(e.currentTarget.value)}
    onkeydown={(e) => {
      e.stopPropagation();
      if (e.key === 'Enter') void oncommit(e.currentTarget.value);
      if (e.key === 'Escape') oncancel();
    }}
  />
{:else}
  <span class="label">{row.label}</span>
  {#if row.host}
    <span class="sub">{row.host.username ? row.host.username + '@' : ''}{row.host.hostname}</span>
    {#if row.host && sessions.hasHost(row.id)}
      <span class="dot live" role="img" aria-label={m.status_connected()}></span>
    {/if}
    <span class="acts">
      <button
        type="button"
        class="act"
        tabindex="-1"
        title={m.tab_terminal()}
        aria-label={m.tab_terminal()}
        onclick={(e) => (e.stopPropagation(), connectHost(row.id, 'terminal'))}
        onpointerdown={(e) => e.stopPropagation()}
        ondblclick={(e) => e.stopPropagation()}><SquareTerminal size={13} /></button
      >
      <button
        type="button"
        class="act"
        tabindex="-1"
        title={m.tab_files()}
        aria-label={m.tab_files()}
        onclick={(e) => (e.stopPropagation(), connectHost(row.id, 'sftp'))}
        onpointerdown={(e) => e.stopPropagation()}
        ondblclick={(e) => e.stopPropagation()}><FolderOpen size={13} /></button
      >
    </span>
  {:else}
    <span class="count tnum">{row.count}</span>
  {/if}
{/if}
