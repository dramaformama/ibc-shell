<script lang="ts">
  import { tick } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { crumbs } from './model';

  let { path, go }: { path: string; go: (p: string) => void } = $props();

  let editing = $state(false);
  let draft = $state('');
  let input = $state<HTMLInputElement>();
  let strip = $state<HTMLElement>();

  const parts = $derived(crumbs(path));

  async function edit() {
    draft = path;
    editing = true;
    await tick();
    input?.select();
  }
  function commit() {
    const p = draft.trim();
    editing = false;
    if (p && p !== path) go(p);
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'Enter') commit();
    else if (e.key === 'Escape') editing = false;
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  // Keep the deepest crumb in view when the path is longer than the strip.
  $effect(() => {
    void path;
    if (strip) strip.scrollLeft = strip.scrollWidth;
  });
</script>

{#if editing}
  <input
    class="sf-path-input mono"
    bind:this={input}
    bind:value={draft}
    onkeydown={key}
    onblur={() => (editing = false)}
    aria-label={m.sftp_path()}
    spellcheck="false"
    autocomplete="off"
  />
{:else}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="sf-crumbs" bind:this={strip} onclick={(e) => e.target === e.currentTarget && edit()}>
    <nav aria-label={m.sftp_path()}>
      {#each parts as c, i (c.path)}
        {#if i > 1 || (i === 1 && parts[0]?.label !== '/')}<span class="sep" aria-hidden="true"
            >/</span
          >{/if}
        <button
          type="button"
          class="crumb"
          class:cur={i === parts.length - 1}
          aria-current={i === parts.length - 1 ? 'page' : undefined}
          onclick={() => go(c.path)}>{c.label}</button
        >
      {/each}
    </nav>
    <button type="button" class="sf-edit-hit" aria-label={m.sftp_edit_path()} onclick={edit}
    ></button>
  </div>
{/if}
