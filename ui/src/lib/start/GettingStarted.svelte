<script lang="ts">
  import { Check, DatabaseBackup, KeyRound, Server } from '@lucide/svelte';
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
      icon: Server,
      done: start.hasServer,
      title: m.start_server_title(),
      hint: m.start_server_hint(),
      action: m.action_new_host(),
      run: () => ui.openHostEditor(null)
    },
    {
      id: 'key',
      icon: KeyRound,
      done: start.hasKey,
      title: m.start_key_title(),
      hint: m.start_key_hint(),
      action: m.start_key_action(),
      run: addKey
    },
    {
      id: 'backup',
      icon: DatabaseBackup,
      done: start.backup,
      title: m.start_backup_title(),
      hint: m.start_backup_hint(),
      action: m.start_backup_action(),
      run: () => (ui.settingsOpen = true)
    }
  ]);
</script>

<section class="start" class:roomy aria-label={m.start_title()}>
  <header>
    <h2>{m.start_title()}</h2>
    <span class="count tnum">{m.start_progress({ done: start.done, total: 3 })}</span>
    <span class="sp"></span>
    <button class="btn btn-ghost" onclick={() => start.dismiss()}>{m.start_hide()}</button>
  </header>
  <ol>
    {#each steps as s, i (s.id)}
      {@const Icon = s.icon}
      <li class:done={s.done}>
        <span class="mark" aria-hidden="true"
          >{#if s.done}<Check size={14} strokeWidth={2.5} />{:else}{i + 1}{/if}</span
        >
        <span class="ic" aria-hidden="true"><Icon size={16} /></span>
        <span class="tx"><b>{s.title}</b><small>{s.hint}</small></span>
        {#if s.done}
          <span class="ok">{m.start_done()}</span>
        {:else}
          <button class="btn" onclick={s.run}>{s.action}</button>
        {/if}
      </li>
    {/each}
  </ol>
</section>

<style>
  .start {
    container-type: inline-size;
    padding: var(--card-pad);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--card-shadow);
    text-align: start;
  }
  .start.roomy {
    width: min(620px, 100%);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }
  .start h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  .count {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
  }
  .sp {
    flex: 1;
  }
  ol {
    display: grid;
    gap: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 24px 34px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .mark {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--border);
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .done .mark {
    background: var(--ok);
    box-shadow: none;
    color: #fff;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 11px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .done .ic {
    background: var(--surface-2);
    color: var(--muted);
  }
  .tx {
    display: grid;
    gap: 1px;
    min-width: 0;
    line-height: 1.35;
  }
  .tx b {
    font-size: 14px;
    font-weight: 600;
  }
  .tx small {
    color: var(--muted);
    font-size: 12px;
  }
  .done .tx b {
    color: var(--muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in oklab, var(--muted) 50%, transparent);
  }
  .ok {
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }
  @container (max-width: 480px) {
    li {
      grid-template-columns: 24px minmax(0, 1fr);
    }
    .ic {
      display: none;
    }
    li > :last-child {
      grid-column: 2;
      justify-self: start;
    }
  }
</style>
