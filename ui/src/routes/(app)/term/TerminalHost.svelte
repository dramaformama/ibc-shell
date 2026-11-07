<script lang="ts">
  import { sessions } from '$lib/stores/sessions.svelte';
  import TerminalView from '$lib/terminal/TerminalView.svelte';

  const terms = $derived(sessions.tabs.filter((t) => t.kind === 'terminal'));
</script>

<!-- Every terminal tab stays mounted (its session keeps running); only the active one is shown. -->
{#each terms as tab (tab.id)}
  <TerminalView {tab} active={tab.id === sessions.activeId} />
{/each}
