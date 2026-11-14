<script lang="ts">
  import { Check, X } from '@lucide/svelte';
  import { api } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import { start } from './start.svelte';

  /** Three first steps. `roomy` is the large version shown when there are no servers yet. */
  let { roomy = false }: { roomy?: boolean } = $props();

  async function addKey() {
    const r = await api.identityImportFile(null);
    if (r.status === 'error') return ui.toast(errorText(r.error), 'error');
    if (r.data) {
      await start.refreshKeys();
      ui.toast(m.start_key_added(), 'info');
    }
  }

  const steps = $derived([
    {
      id: 'server',
      done: start.hasServer,
      title: m.start_server_title(),
      hint: m.start_server_hint(),
      action: m.action_new_host(),
      run: () => ui.openHostEditor(null)
    },
    {
      id: 'key',
      done: start.hasKey,
      title: m.start_key_title(),
      hint: m.start_key_hint(),
      action: m.start_key_action(),
      run: addKey
    },
    {
      id: 'backup',
      done: start.backup,
      title: m.start_backup_title(),
      hint: m.start_backup_hint(),
      action: m.start_backup_action(),
      run: () => ui.openSettings('backup')
    }
  ]);
  const next = $derived(steps.find((s) => !s.done));
</script>

{#if roomy}
  <section class="start roomy" aria-label={m.start_title()}>
    <header>
      <h2>{m.start_title()}</h2>
      <span class="count tnum">{m.start_progress({ done: start.done, total: 3 })}</span>
      <span class="sp"></span>
      <button class="btn btn-ghost btn-sm" onclick={() => start.dismiss()}>{m.start_hide()}</button>
    </header>
    <ol>
      {#each steps as s, i (s.id)}
        <li class:done={s.done}>
          <span class="mark" aria-hidden="true"
            >{#if s.done}<Check size={12} strokeWidth={3} />{:else}{i + 1}{/if}</span
          >
          <span class="tx"><b>{s.title}</b><small>{s.hint}</small></span>
          {#if s.done}
            <span class="ok">{m.start_done()}</span>
          {:else}
            <button class="btn btn-sm" onclick={s.run}>{s.action}</button>
          {/if}
        </li>
      {/each}
    </ol>
  </section>
{:else if next}
  <!-- One quiet line above the list: the next step only. -->
  <aside class="line" aria-label={m.start_title()}>
    <span class="steps" aria-hidden="true">
      {#each steps as s (s.id)}<i class:on={s.done}></i>{/each}
    </span>
    <span class="tx"
      ><b>{m.start_title()} · {m.start_progress({ done: start.done, total: 3 })}</b>
      {next.title}</span
    >
    <button class="btn btn-sm" onclick={next.run}>{next.action}</button>
    <button
      class="icon-btn"
      onclick={() => start.dismiss()}
      aria-label={m.start_hide()}
      title={m.start_hide()}><X size={14} /></button
    >
  </aside>
{/if}

<style>
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
    padding: 6px 6px 6px 12px;
    border-radius: var(--r-control);
    background: var(--fill);
  }
  .line .tx {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .line .tx b {
    color: var(--text);
    font-weight: 600;
    margin-right: 6px;
  }
  .line .icon-btn {
    width: 24px;
    height: 24px;
  }
  .steps {
    display: inline-flex;
    gap: 3px;
    flex: none;
  }
  .steps i {
    width: 14px;
    height: 4px;
    border-radius: 2px;
    background: var(--fill-2);
  }
  .steps i.on {
    background: var(--accent);
  }
  .start {
    container-type: inline-size;
    width: min(560px, 100%);
    padding: 14px 16px 6px;
    border-radius: var(--r-card);
    background: var(--fill);
    text-align: start;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .start h2 {
    margin: 0;
    font-size: var(--fs-body);
    font-weight: 600;
    color: var(--text);
  }
  .count {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .sp {
    flex: 1;
  }
  ol {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
  }
  li + li {
    border-top: 0.5px solid var(--sep);
  }
  .mark {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--muted);
    font-size: var(--fs-cap);
    font-weight: 600;
  }
  .done .mark {
    background: var(--accent-fill);
    box-shadow: none;
    color: var(--on-accent);
  }
  .tx {
    display: grid;
    gap: 1px;
    min-width: 0;
    line-height: 1.35;
  }
  li .tx b {
    font-weight: 500;
  }
  li .tx small {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .done .tx b {
    color: var(--muted);
  }
  .ok {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  @container (max-width: 420px) {
    li {
      grid-template-columns: 20px minmax(0, 1fr);
    }
    li > :last-child {
      grid-column: 2;
      justify-self: start;
    }
  }
</style>
