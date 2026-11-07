<script lang="ts">
  import {
    ChevronRight,
    CornerDownLeft,
    FolderPlus,
    Pencil,
    Play,
    Plus,
    Search,
    Send,
    Trash2,
    Zap
  } from '@lucide/svelte';
  import type { Snippet } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { frequent, parseVars, sections } from './model';
  import { snippets } from './snippets.svelte';
  import './snippets-panel.css';

  let q = $state('');
  const secs = $derived(sections(snippets.items, snippets.layout, q));
  const top = $derived(q ? [] : frequent(snippets.items, snippets.layout));
  let renaming = $state<string | null>(null);
  let renameValue = $state('');
  let dragId = $state<string | null>(null);
  let dragGroup = $state<string | null>(null);
  let over = $state<string | null>(null);

  const firstLine = (b: string) => b.split('\n').find((l) => l.trim()) ?? '';

  function newFolder() {
    const id = snippets.addGroup(m.snip_folder_default());
    renaming = id;
    renameValue = m.snip_folder_default();
  }
  function commitRename() {
    if (renaming && renameValue.trim()) snippets.renameGroup(renaming, renameValue.trim());
    renaming = null;
  }

  // ---- drag & drop: snippets onto rows / folder headers, folders onto folder headers ----
  function dragSnippet(e: DragEvent, s: Snippet) {
    dragId = s.id;
    e.dataTransfer?.setData('text/plain', s.name);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function dragFolder(e: DragEvent, id: string) {
    dragGroup = id;
    e.dataTransfer?.setData('text/plain', id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function allow(e: DragEvent, key: string) {
    if (!dragId && !dragGroup) return;
    e.preventDefault();
    over = key;
  }
  function dropOnRow(e: DragEvent, group: string | null, before: string) {
    e.preventDefault();
    if (dragId) snippets.move(dragId, group, before);
    end();
  }
  function dropOnFolder(e: DragEvent, group: string | null) {
    e.preventDefault();
    if (dragId) snippets.move(dragId, group, null);
    else if (dragGroup && group && dragGroup !== group) {
      const gs = snippets.layout.groups;
      const from = gs.findIndex((g) => g.id === dragGroup);
      const to = gs.findIndex((g) => g.id === group);
      for (let i = 0; i < Math.abs(to - from); i++)
        snippets.moveGroup(dragGroup, to > from ? 1 : -1);
    }
    end();
  }
  function end() {
    dragId = dragGroup = over = null;
  }
</script>

<div class="snips">
  <div class="toolbar">
    <label class="search"
      ><Search size={14} /><input
        type="search"
        bind:value={q}
        placeholder={m.snip_search()}
        aria-label={m.snip_search()}
      /></label
    >
    <button
      class="icon-btn"
      onclick={newFolder}
      aria-label={m.snip_new_folder()}
      title={m.snip_new_folder()}><FolderPlus size={16} /></button
    >
    <button
      class="icon-btn add"
      onclick={() => (snippets.editing = { snippet: null, group: null })}
      aria-label={m.snip_new()}
      title={m.snip_new()}><Plus size={16} /></button
    >
  </div>

  {#if !snippets.items.length && snippets.loaded}
    <div class="empty">
      <span class="empty-icon"><Zap size={22} /></span>
      <h3>{m.snip_empty_title()}</h3>
      <p>{m.snip_empty_hint()}</p>
      <button
        class="btn btn-primary btn-sm"
        onclick={() => (snippets.editing = { snippet: null, group: null })}
        ><Plus size={14} />{m.snip_new()}</button
      >
    </div>
  {:else}
    {#if top.length}
      <div class="frequent" aria-label={m.snip_frequent()}>
        <span class="cap">{m.snip_frequent()}</span>
        <div class="pills">
          {#each top as s (s.id)}
            <button class="pill" onclick={() => snippets.run(s)} title={s.body}
              ><Play size={11} />{s.name}</button
            >
          {/each}
        </div>
      </div>
    {/if}

    <div class="list">
      {#each secs as sec (sec.group?.id ?? '_')}
        {@const gid = sec.group?.id ?? null}
        {@const closed = !!gid && !q && snippets.layout.collapsed.includes(gid)}
        {#if sec.group || snippets.layout.groups.length}
          <div
            class="folder"
            class:over={over === 'f:' + (gid ?? '_')}
            role="group"
            draggable={!!gid && renaming !== gid}
            ondragstart={(e) => gid && dragFolder(e, gid)}
            ondragover={(e) => allow(e, 'f:' + (gid ?? '_'))}
            ondragleave={() => (over = null)}
            ondrop={(e) => dropOnFolder(e, gid)}
            ondragend={end}
          >
            {#if gid && renaming === gid}
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="input rename"
                bind:value={renameValue}
                autofocus
                aria-label={m.snip_folder_name()}
                onblur={commitRename}
                onkeydown={(e) => {
                  if (e.key === 'Enter') commitRename();
                  if (e.key === 'Escape') renaming = null;
                }}
              />
            {:else}
              <button
                class="fold"
                aria-expanded={!closed}
                onclick={() => gid && snippets.toggleGroup(gid)}
                disabled={!gid}
                ><span class="chev" class:open={!closed}><ChevronRight size={13} /></span><span
                  class="fname">{sec.group?.name ?? m.snip_no_folder()}</span
                ><span class="count tnum">{sec.items.length}</span></button
              >
              {#if gid}
                <span class="facts">
                  <button
                    class="mini"
                    onclick={() => (snippets.editing = { snippet: null, group: gid })}
                    aria-label={m.snip_new()}
                    title={m.snip_new()}><Plus size={13} /></button
                  >
                  <button
                    class="mini"
                    onclick={() => ((renaming = gid), (renameValue = sec.group?.name ?? ''))}
                    aria-label={m.menu_rename()}
                    title={m.menu_rename()}><Pencil size={12} /></button
                  >
                  <button
                    class="mini"
                    onclick={() => snippets.removeGroup(gid)}
                    aria-label={m.snip_delete_folder()}
                    title={m.snip_delete_folder()}><Trash2 size={12} /></button
                  >
                </span>
              {/if}
            {/if}
          </div>
        {/if}
        {#if !closed}
          {#each sec.items as s (s.id)}
            {@const hasVars = parseVars(s.body).length > 0}
            <div
              class="row"
              class:over={over === 's:' + s.id}
              class:dragging={dragId === s.id}
              role="listitem"
              draggable="true"
              ondragstart={(e) => dragSnippet(e, s)}
              ondragover={(e) => allow(e, 's:' + s.id)}
              ondragleave={() => (over = null)}
              ondrop={(e) => dropOnRow(e, gid, s.id)}
              ondragend={end}
            >
              <button class="main" onclick={() => snippets.run(s)} title={m.snip_run_hint()}>
                <span class="name"
                  >{s.name}{#if hasVars}<span class="tag">{'{…}'}</span>{/if}</span
                >
                <span class="cmd mono">{firstLine(s.body)}</span>
              </button>
              <span class="acts">
                <button
                  class="mini"
                  onclick={() => snippets.run(s, 'here', false)}
                  aria-label={m.snip_insert()}
                  title={m.snip_insert()}><CornerDownLeft size={13} /></button
                >
                <button
                  class="mini"
                  onclick={() => snippets.run(s, 'many')}
                  aria-label={m.snip_run_many()}
                  title={m.snip_run_many()}><Send size={13} /></button
                >
                <button
                  class="mini"
                  onclick={() => (snippets.editing = { snippet: s, group: null })}
                  aria-label={m.snip_edit()}
                  title={m.snip_edit()}><Pencil size={12} /></button
                >
              </span>
            </div>
          {:else}
            {#if gid && !q}
              <div
                class="drop-hint"
                class:over={over === 'f:' + gid}
                role="listitem"
                ondragover={(e) => allow(e, 'f:' + gid)}
                ondrop={(e) => dropOnFolder(e, gid)}
              >
                {m.snip_folder_empty()}
              </div>
            {/if}
          {/each}
        {/if}
      {/each}
      {#if q && !secs.length}
        <p class="none">{m.dash_no_results()}</p>
      {/if}
    </div>
  {/if}
</div>
