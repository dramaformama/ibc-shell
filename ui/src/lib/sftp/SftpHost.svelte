<script lang="ts">
  import { sessions } from '$lib/stores/sessions.svelte';
  import SftpScreen from './SftpScreen.svelte';
  const tabs = $derived(sessions.tabs.filter((t) => t.kind === 'sftp'));
</script>

<!-- File views, like terminals, retain their connection and navigation until the server closes. -->
{#each tabs as tab (tab.id)}
  <div class="file-view" hidden={sessions.activeId !== tab.id}>
    <SftpScreen {tab} />
  </div>
{/each}

<style>
  .file-view {
    position: absolute;
    inset: 0;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .file-view[hidden] {
    display: none;
  }
</style>
