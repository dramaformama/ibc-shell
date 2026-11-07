<script lang="ts">
  import Inspector from '$lib/stats/Inspector.svelte';
  import Dashboard from '$lib/stats/Dashboard.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';

  const tab = $derived(sessions.active);
  const group = $derived(sessions.groups.find((g) => g.key === sessions.activeGroup));
  // SFTP is lazy-loaded (ui.md performance rules); one screen per SFTP tab.
  const loadSftp = () => import('$lib/sftp/SftpHost.svelte');
</script>

<!-- Terminal tabs are mounted by TerminalHost (lazy); SFTP content is mounted here by sftp-ui. -->
<div class="stage" data-tab-id={tab?.id} data-tab-kind={tab?.kind}>
  <div class="content">
    {#await import('./term/TerminalHost.svelte') then { default: TerminalHost }}
      <TerminalHost />
    {/await}
    {#if sessions.tabs.some((t) => t.kind === 'sftp')}
      {#await loadSftp() then { default: SftpHost }}
        <SftpHost />
      {/await}
    {/if}
    {#if !tab}
      <Dashboard />
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
</style>
