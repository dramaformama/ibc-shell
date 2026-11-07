<script lang="ts">
  import '../app.css';
  import '$lib/forms.css';
  import { onMount } from 'svelte';
  import Shell from '$lib/components/Shell.svelte';
  import VaultScreen from '$lib/components/VaultScreen.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { vault } from '$lib/stores/vault.svelte';

  let { children } = $props();

  onMount(() => void vault.refresh());

  // Load data when the vault opens; wipe it when it locks.
  $effect(() => {
    if (vault.state === 'Unlocked') {
      void settings.load();
      void tree.load();
    } else if (vault.state === 'Locked') {
      tree.data = { folders: [], hosts: [] };
      tree.loaded = false;
      sessions.clear();
    }
  });
</script>

{#if vault.state === 'Unlocked'}
  <Shell>{@render children()}</Shell>
{:else if vault.state !== 'Loading'}
  <VaultScreen />
{/if}
