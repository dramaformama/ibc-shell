<script lang="ts">
  import {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Eye,
    EyeOff,
    FolderPlus,
    RefreshCw,
    Search
  } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import Breadcrumb from './Breadcrumb.svelte';
  import { dnd } from './dnd';
  import FileTable from './FileTable.svelte';
  import { joinPath, type Actions, type Source } from './model';
  import type { Pane } from './paneState.svelte';

  let {
    pane,
    id,
    sources,
    sourceValue,
    active,
    hover = false,
    act,
    onSource,
    onDrop,
    onFocus
  }: {
    pane: Pane;
    id: 'left' | 'right';
    sources: Source[];
    sourceValue: string;
    active: boolean;
    hover?: boolean;
    act: Actions;
    onSource: (value: string) => void;
    onDrop: (from: 'left' | 'right', names: string[], dstDir: string) => void;
    onFocus: () => void;
  } = $props();

  let dropName = $state<string | null>(null);
  let dropping = $state(false);

  function over(e: DragEvent) {
    const src = dnd.source;
    if (!src || src.pane === id) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    dropping = true;
    dropName = (e.target as HTMLElement).closest<HTMLElement>('[data-dir]')?.dataset.dir ?? null;
  }
  function leave(e: DragEvent) {
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node | null)) {
      dropping = false;
      dropName = null;
    }
  }
  function drop(e: DragEvent) {
    const src = dnd.source;
    const dir = dropName;
    dropping = false;
    dropName = null;
    if (!src || src.pane === id) return;
    e.preventDefault();
    onDrop(src.pane, src.names, dir ? joinPath(pane.path, dir) : pane.path);
  }
</script>

<section
  class="sf-pane"
  class:active
  class:dropping={dropping || hover}
  aria-label={id === 'left' ? m.sftp_pane_left() : m.sftp_pane_right()}
  onfocusin={onFocus}
  onpointerdown={onFocus}
  ondragover={over}
  ondragleave={leave}
  ondrop={drop}
>
  <header class="sf-bar">
    <div class="sf-line">
      <select
        class="sf-source"
        value={sourceValue}
        aria-label={m.sftp_source()}
        onchange={(e) => onSource(e.currentTarget.value)}
      >
        {#each sources as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>
      <div class="sf-nav">
        <button
          class="icon-btn"
          title={m.sftp_back()}
          aria-label={m.sftp_back()}
          disabled={!pane.back.length}
          onclick={() => pane.goBack()}><ArrowLeft size={15} /></button
        >
        <button
          class="icon-btn"
          title={m.sftp_forward()}
          aria-label={m.sftp_forward()}
          disabled={!pane.forward.length}
          onclick={() => pane.goForward()}><ArrowRight size={15} /></button
        >
        <button
          class="icon-btn"
          title={m.sftp_up()}
          aria-label={m.sftp_up()}
          onclick={() => pane.up()}><ArrowUp size={15} /></button
        >
      </div>
      <Breadcrumb path={pane.path} go={(p) => void pane.load(p)} />
    </div>
    <div class="sf-line sf-tools">
      <label class="sf-filter">
        <Search size={13} />
        <input
          bind:value={pane.filter}
          placeholder={m.sftp_filter()}
          aria-label={m.sftp_filter()}
          spellcheck="false"
        />
      </label>
      <button
        class="icon-btn"
        title={m.sftp_refresh()}
        aria-label={m.sftp_refresh()}
        onclick={() => pane.refresh()}
        ><RefreshCw size={14} class={pane.loading ? 'spin' : ''} /></button
      >
      <button
        class="icon-btn"
        title={m.action_new_folder()}
        aria-label={m.action_new_folder()}
        onclick={act.newFolder}><FolderPlus size={15} /></button
      >
      <button
        class="icon-btn"
        class:on={pane.showHidden}
        title={m.sftp_hidden()}
        aria-label={m.sftp_hidden()}
        aria-pressed={pane.showHidden}
        onclick={() => (pane.showHidden = !pane.showHidden)}
      >
        {#if pane.showHidden}<Eye size={15} />{:else}<EyeOff size={15} />{/if}
      </button>
    </div>
  </header>
  <FileTable {pane} {id} {dropName} {act} />
  <footer class="sf-foot tnum">
    {#if pane.selected.length}{m.sftp_selected({
        count: pane.selected.length,
        total: pane.view.length
      })}
    {:else}{m.sftp_items({ count: pane.view.length })}{/if}
  </footer>
</section>
