<script lang="ts">
  import Dashboard from '$lib/stats/Dashboard.svelte';
  import FleetCerts from '$lib/fleetpages/FleetCerts.svelte';
  import FleetDocker from '$lib/fleetpages/FleetDocker.svelte';
  import SnippetsPage from '$lib/fleetpages/SnippetsPage.svelte';
  import SidePanel from '$lib/stats/SidePanel.svelte';
  import StatusView from '$lib/stats/StatusView.svelte';
  import RunDialog from '$lib/snippets/RunDialog.svelte';
  import FolderDialog from '$lib/snippets/FolderDialog.svelte';
  import SnippetEditor from '$lib/snippets/SnippetEditor.svelte';
  import { snippets } from '$lib/snippets/snippets.svelte';
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
      {#if ui.page === 'docker'}<FleetDocker />{:else if ui.page === 'domains'}<FleetCerts
        />{:else if ui.page === 'snippets'}<SnippetsPage />{:else}<Dashboard />{/if}
    {:else if tab.kind === 'status'}
      {#key tab.id}<StatusView {tab} />{/key}
    {/if}
  </div>
  {#if tab?.kind === 'terminal' && !tab.local && group && ui.inspectorOpen}
    <SidePanel {group} />
  {/if}
</div>

{#if snippets.pending}
  {#key snippets.pending}<RunDialog run={snippets.pending} />{/key}
{/if}
{#if snippets.editing}
  <SnippetEditor />
{/if}
<FolderDialog />

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
