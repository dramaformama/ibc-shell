<script lang="ts">
  import EmptyState from '$lib/components/EmptyState.svelte';
  import {
    CornerDownLeft,
    Folder,
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
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { pluralForm } from '$lib/stats/format';
  import { frequent, parseVars, sections } from './model';
  import { snippets } from './snippets.svelte';
  import './snippets-panel.css';

  let q = $state('');
  const selected = $derived(
    snippets.layout.groups.some((g) => g.id === snippets.selectedGroup)
      ? snippets.selectedGroup
      : null
  );
  const all = $derived(sections(snippets.items, snippets.layout, q));
  // A picked folder card narrows the list to that folder; search always looks everywhere.
  const secs = $derived(selected && !q ? all.filter((x) => x.group?.id === selected) : all);
  const top = $derived(q ? [] : frequent(snippets.items, snippets.layout));
  let dragId = $state<string | null>(null);
  let dragGroup = $state<string | null>(null);
  let over = $state<string | null>(null);

  const firstLine = (b: string) => b.split('\n').find((l) => l.trim()) ?? '';
  const countOf = (id: string) =>
    snippets.items.filter((x) => snippets.layout.of[x.id] === id).length;
  const countText = (count: number) => {
    const f = pluralForm(count, getLocale());
    return f === 'one'
      ? m.snip_count_one({ count })
      : f === 'few'
        ? m.snip_count_few({ count })
        : m.snip_count_many({ count });
  };

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
      onclick={() => (snippets.folderDialog = { id: null })}
      aria-label={m.snip_new_folder()}
      title={m.snip_new_folder()}><FolderPlus size={16} /></button
    >
    <button
      class="icon-btn add"
      onclick={() => (snippets.editing = { snippet: null, group: selected })}
      aria-label={m.snip_new()}
      title={m.snip_new()}><Plus size={16} /></button
    >
  </div>

  {#if snippets.layout.groups.length}
    <div class="folders" role="group" aria-label={m.snip_folders()}>
      {#each snippets.layout.groups as g (g.id)}
        <div
          class="fcard"
          class:on={selected === g.id}
          class:over={over === 'f:' + g.id}
          role="group"
          draggable="true"
          ondragstart={(e) => dragFolder(e, g.id)}
          ondragover={(e) => allow(e, 'f:' + g.id)}
          ondragleave={() => (over = null)}
          ondrop={(e) => dropOnFolder(e, g.id)}
          ondragend={end}
        >
          <button
            class="fmain"
            aria-pressed={selected === g.id}
            title={m.snip_folder_filter()}
            onclick={() => (snippets.selectedGroup = selected === g.id ? null : g.id)}
            ondblclick={() => (snippets.folderDialog = { id: g.id })}
          >
            <span class="badge"><Folder size={16} /></span>
            <span class="ftext">
              <span class="fname">{g.name}</span>
              <span class="fmeta tnum">{countText(countOf(g.id))}</span>
            </span>
          </button>
          <span class="facts">
            <button
              class="mini"
              onclick={() => (snippets.folderDialog = { id: g.id })}
              aria-label={m.menu_rename()}
              title={m.menu_rename()}><Pencil size={12} /></button
            >
            <button
              class="mini"
              onclick={() => (snippets.folderDelete = g.id)}
              aria-label={m.snip_delete_folder()}
              title={m.snip_delete_folder()}><Trash2 size={12} /></button
            >
          </span>
        </div>
      {/each}
    </div>
  {/if}

  {#if !snippets.items.length && snippets.loaded}
    <EmptyState icon={Zap} title={m.snip_empty_title()} hint={m.snip_empty_hint()}>
      <button
        class="btn btn-primary btn-sm"
        onclick={() => (snippets.editing = { snippet: null, group: null })}
        ><Plus size={14} />{m.snip_new()}</button
      >
    </EmptyState>
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
        {@const fname = !selected ? (sec.group?.name ?? null) : null}
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
                >{s.name}{#if hasVars}<span class="tag">{'{…}'}</span>{/if}{#if fname}<span
                    class="tag in">{fname}</span
                  >{/if}</span
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
          {#if gid && !q && selected}
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
      {/each}
      {#if q && !secs.length}
        <p class="none">{m.dash_no_results()}</p>
      {/if}
    </div>
  {/if}
</div>
