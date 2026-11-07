<script lang="ts">
  import '../app.css';
  import '$lib/forms.css';
  import { onMount } from 'svelte';
  import Shell from '$lib/components/Shell.svelte';
  import VaultScreen from '$lib/components/VaultScreen.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { recent } from '$lib/stores/recent.svelte';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { geo } from '$lib/map/geoip.svelte';
  import { connectInBackground } from '$lib/stores/actions';
  import { snippets } from '$lib/snippets/snippets.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { watchIdle } from '$lib/util/autolock';

  let { children } = $props();

  onMount(() => void vault.refresh());

  // Load data when the vault opens; wipe it when it locks.
  $effect(() => {
    if (vault.state === 'Unlocked') {
      void settings.load().then(() => settings.checkUpdate());
      void Promise.all([tree.load(), lastSeen.load()]).then(() => {
        for (const id of lastSeen.watched) if (tree.host(id)) connectInBackground(id);
      });
      void recent.load();
      void geo.load();
      void snippets.load();
    } else if (vault.state === 'Locked') {
      tree.data = { folders: [], hosts: [] };
      tree.loaded = false;
      sessions.clear();
      stats.clear();
      lastSeen.clear();
      geo.clear();
      snippets.clear();
    }
  });

  // Idle auto-lock while unlocked.
  $effect(() => {
    if (vault.state !== 'Unlocked') return;
    return watchIdle(settings.autolockMinutes, () => void vault.lock());
  });
</script>

{#if vault.state === 'Unlocked'}
  <Shell>{@render children()}</Shell>
{:else if vault.state !== 'Loading'}
  <VaultScreen />
{/if}
