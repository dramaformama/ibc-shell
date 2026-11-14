<script lang="ts">
  import { scrollEdge } from '$lib/util/scroll-edge.svelte';
  import { Eye, FolderOpen, SquareTerminal, Zap } from '@lucide/svelte';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { switchKind } from '$lib/stores/actions';
  import { sessions, type Tab } from '$lib/stores/sessions.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import ServicesPane from '$lib/services/ServicesPane.svelte';
  import MetricsPane from './MetricsPane.svelte';
  import NotesCard from './NotesCard.svelte';
  import ForwardsCard from '$lib/forwards/ForwardsCard.svelte';
  import KeyLoginCard from '$lib/keys/KeyLoginCard.svelte';

  /** A server's own page: identity, quick actions and every metric as a card grid. */
  let { tab }: { tab: Tab } = $props();
  const group = $derived(sessions.groupOf(tab.id));
  const host = $derived(tab.hostId ? tree.host(tab.hostId) : null);
  const phase = $derived(group ? sessions.groupPhase(group) : 'disconnected');
  const sid = $derived(group ? sessions.groupSession(group) : null);
</script>

{#if group}
  <section class="status" aria-label={m.tab_status()} use:scrollEdge>
    <header>
      <span class="badge"><Flag code={group.flag} size={40} /></span>
      <div class="id">
        <h1>{group.label}</h1>
        <p>
          <span class="dot" data-phase={phase}></span>
          {#if host}{host.username ? `${host.username}@` : ''}{host.hostname}{host.port &&
            host.port !== 22
              ? `:${host.port}`
              : ''}{/if}
        </p>
      </div>
      <div class="acts">
        {#if host}
          <button
            class="btn"
            aria-pressed={lastSeen.isWatched(host.id)}
            onclick={() => lastSeen.setWatched(host.id, !lastSeen.isWatched(host.id))}
            title={m.stat_watch_hint()}><Eye size={14} />{m.stat_watch()}</button
          >
        {/if}
        <button
          class="btn"
          onclick={() => {
            ui.panelTab = 'snippets';
            if (!ui.inspectorOpen) ui.toggleInspector();
            switchKind('terminal');
          }}><Zap size={14} />{m.panel_snippets()}</button
        >
        <button class="btn" onclick={() => switchKind('sftp')}
          ><FolderOpen size={14} />{m.tab_files()}</button
        >
        <button class="btn btn-primary" onclick={() => switchKind('terminal')}
          ><SquareTerminal size={14} />{m.tab_terminal()}</button
        >
      </div>
    </header>
    <MetricsPane {group} wide />
    {#if sid}<ServicesPane {sid} />{/if}
    {#if host}<KeyLoginCard hostId={host.id} {sid} />{/if}
    {#if host}<ForwardsCard hostId={host.id} {sid} />{/if}
    {#if host}<NotesCard hostId={host.id} />{/if}
  </section>
{/if}

<style>
  .status {
    container-type: inline-size;
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: var(--page-pt) var(--page-px) 40px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 22px;
  }
  .badge {
    display: grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 40px;
  }
  .id {
    flex: 1;
    min-width: 0;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 3px;
    font-size: var(--fs-large);
    font-weight: 700;
    letter-spacing: -0.025em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .id p {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
  .acts .btn[aria-pressed='true'] {
    color: var(--accent);
  }
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  @container (max-width: 900px) {
    header {
      flex-wrap: wrap;
    }
    .acts {
      width: 100%;
      justify-content: flex-start;
    }
  }
  @media (max-width: 760px) {
    .status {
      padding: 20px 16px 32px;
    }
  }
</style>
