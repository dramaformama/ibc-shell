<script lang="ts">
  import { Plus, Search } from '@lucide/svelte';
  import Inspector from '$lib/stats/Inspector.svelte';
  import Dashboard from '$lib/stats/Dashboard.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { mod } from '$lib/util/platform';

  const tab = $derived(sessions.active);
  const group = $derived(sessions.groups.find((g) => g.key === sessions.activeGroup));
  // SFTP is lazy-loaded (ui.md performance rules); one screen per SFTP tab.
  const loadSftp = () => import('$lib/sftp/SftpScreen.svelte');
</script>

<!-- Terminal tabs are mounted by TerminalHost (lazy); SFTP content is mounted here by sftp-ui. -->
<div class="stage" data-tab-id={tab?.id} data-tab-kind={tab?.kind}>
  <div class="content">
    {#await import('./term/TerminalHost.svelte') then { default: TerminalHost }}
      <TerminalHost />
    {/await}
    {#if tab?.kind === 'sftp'}
      {#await loadSftp() then { default: SftpScreen }}
        {#key tab.id}<SftpScreen {tab} />{/key}
      {/await}
    {:else if !tab && sessions.groups.length}
      <Dashboard />
    {:else if !tab}
      <div class="welcome">
        <img class="mark" src="/favicon.svg" alt="" width="56" height="56" />
        <h1>{m.welcome_title()}</h1>
        <p>{m.welcome_hint()}</p>
        <div class="actions">
          <button class="btn btn-primary" onclick={() => ui.openHostEditor(null)}
            ><Plus size={15} />{m.welcome_new()}</button
          >
          <button class="btn" onclick={() => (ui.paletteOpen = true)}
            ><Search size={14} />{m.palette_open()}
            <span class="kbd">{mod('K')}</span></button
          >
        </div>
      </div>
    {/if}
  </div>
  {#if tab?.kind === 'terminal' && group && ui.inspectorOpen}
    <Inspector {group} />
  {/if}
</div>

<style>
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    overflow: hidden;
  }
  .content {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .stage[data-tab-kind='sftp'] .content {
    display: flex;
  }
  .mark {
    width: 72px;
    height: 72px;
    margin-bottom: 14px;
    border-radius: 18px;
    box-shadow: var(--shadow);
  }
  .welcome {
    display: grid;
    gap: 8px;
    justify-items: center;
    text-align: center;
    padding: 24px;
  }
  h1 {
    margin: 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.025em;
    text-wrap: balance;
  }
  p {
    margin: 0 0 14px;
    color: var(--muted);
    max-width: 40ch;
    text-wrap: pretty;
  }
  .kbd {
    display: inline-flex;
    align-items: center;
    gap: 1px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }
</style>
