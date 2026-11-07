<script lang="ts">
  import { FolderOpen, SquareTerminal, X } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type TabPhase } from '$lib/stores/sessions.svelte';

  const phaseText = (p: TabPhase) =>
    ({
      connecting: m.phase_connecting(),
      connected: m.phase_connected(),
      failed: m.phase_failed(),
      disconnected: m.phase_disconnected()
    })[p];
</script>

{#if sessions.tabs.length}
  <section class="open" aria-label={m.sidebar_open()}>
    <h2>{m.sidebar_open()}<span class="count">{sessions.groups.length}</span></h2>
    <ul>
      {#each sessions.groups as g (g.key)}
        {@const phase = sessions.groupPhase(g)}
        {@const on = g.key === sessions.activeGroup}
        <li class:active={on}>
          <button
            class="row"
            aria-current={on ? 'page' : undefined}
            onclick={() => sessions.select(g.current.id)}
          >
            <span class="dot" data-phase={phase} title={phaseText(phase)}></span>
            <Flag code={g.flag} size={16} />
            <span class="name">{g.label}</span>
            <span class="kind">
              {#each g.tabs as t (t.id)}
                {#if t.kind === 'sftp'}<FolderOpen
                    size={13}
                    aria-label={m.tab_files()}
                  />{:else}<SquareTerminal size={13} aria-label={m.tab_terminal()} />{/if}
              {/each}
            </span>
          </button>
          <button
            class="close"
            aria-label={m.tab_disconnect()}
            title={m.tab_disconnect()}
            onclick={() => sessions.closeGroup(g.key)}><X size={12} /></button
          >
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .open {
    padding: 0 8px 6px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 4px 0 4px;
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .count {
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
  }
  ul {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--r-control);
    transition: background-color 120ms var(--ease);
  }
  li:hover {
    background: var(--hover);
  }
  li.active {
    background: var(--accent-soft);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 30px 0 8px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  li.active .row {
    font-weight: 500;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    display: inline-flex;
    gap: 3px;
    color: var(--muted);
  }
  li.active .kind {
    color: var(--accent);
  }
  .close {
    position: absolute;
    right: 4px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    opacity: 0;
    cursor: pointer;
  }
  li:hover .close,
  .close:focus-visible {
    opacity: 1;
  }
  li:hover .kind {
    visibility: hidden;
  }
  .close:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
