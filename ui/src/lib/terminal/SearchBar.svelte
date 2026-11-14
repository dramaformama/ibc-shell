<script lang="ts">
  import { ArrowDown, ArrowUp, CaseSensitive, X } from '@lucide/svelte';
  import type { SearchAddon } from '@xterm/addon-search';
  import { onMount } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';

  let { search, onclose }: { search: SearchAddon; onclose: () => void } = $props();

  let q = $state('');
  let matchCase = $state(false);
  let result = $state<{ index: number; count: number } | null>(null);
  let input: HTMLInputElement;

  const opts = $derived({
    caseSensitive: matchCase,
    decorations: {
      matchBackground: '#d9a40a40',
      matchOverviewRuler: '#d9a40a',
      activeMatchBackground: '#3a66d680',
      activeMatchColorOverviewRuler: '#3a66d6'
    }
  });

  onMount(() => {
    input.focus();
    input.select();
    const sub = search.onDidChangeResults((r) => {
      result = { index: r.resultIndex, count: r.resultCount };
    });
    return () => {
      sub.dispose();
      search.clearDecorations();
    };
  });

  function run(dir: 'next' | 'prev', incremental = false) {
    if (!q) {
      search.clearDecorations();
      result = null;
      return;
    }
    if (dir === 'next') search.findNext(q, { ...opts, incremental });
    else search.findPrevious(q, opts);
  }

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(e.shiftKey ? 'prev' : 'next');
    }
  }

  const none = $derived(!!q && result?.count === 0);
</script>

<div class="bar" role="search">
  <input
    bind:this={input}
    bind:value={q}
    class="q"
    class:none
    type="text"
    spellcheck="false"
    autocomplete="off"
    placeholder={m.term_find()}
    aria-label={m.term_find()}
    oninput={() => run('next', true)}
    {onkeydown}
  />
  <span class="count tnum" aria-live="polite">
    {#if none}{m.term_find_none()}{:else if result && q && result.count > 0}{result.index +
        1}/{result.count}{/if}
  </span>
  <button
    class="icon-btn"
    class:on={matchCase}
    aria-pressed={matchCase}
    aria-label={m.term_find_case()}
    title={m.term_find_case()}
    onclick={() => {
      matchCase = !matchCase;
      queueMicrotask(() => run('next', true));
    }}><CaseSensitive size={16} /></button
  >
  <button
    class="icon-btn"
    aria-label={m.term_find_prev()}
    title={m.term_find_prev()}
    onclick={() => run('prev')}><ArrowUp size={14} /></button
  >
  <button
    class="icon-btn"
    aria-label={m.term_find_next()}
    title={m.term_find_next()}
    onclick={() => run('next')}><ArrowDown size={14} /></button
  >
  <button
    class="icon-btn"
    aria-label={m.term_find_close()}
    title={m.term_find_close()}
    onclick={onclose}><X size={14} /></button
  >
</div>

<style>
  .bar {
    position: absolute;
    top: 10px;
    right: 14px;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 4px 4px 10px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-panel);
    background: var(--surface);
    box-shadow: var(--shadow);
    animation: drop 140ms var(--ease);
  }
  .q {
    width: 168px;
    height: 24px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .q::placeholder {
    color: var(--muted);
  }
  .q.none {
    color: var(--danger);
  }
  .count {
    min-width: 5ch;
    padding: 0 6px;
    text-align: right;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .icon-btn.on {
    background: var(--accent-soft);
    color: var(--accent);
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  @media (max-width: 480px) {
    .q {
      width: 110px;
    }
  }
</style>
