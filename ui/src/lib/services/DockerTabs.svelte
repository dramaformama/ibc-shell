<script lang="ts">
  import type { DockerReport } from '$lib/api';
  import DockerCompose from './DockerCompose.svelte';
  import DockerEvents from './DockerEvents.svelte';
  import DockerImages from './DockerImages.svelte';
  import DockerNetworks from './DockerNetworks.svelte';
  import DockerRows from './DockerRows.svelte';
  import DockerVolumes from './DockerVolumes.svelte';
  import type { DockerTab } from './docker-tabs';

  /** Body of one Docker tab for one server; shared by the server card and the Docker page. */
  let { sid, report, tab }: { sid: string; report: DockerReport | null; tab: DockerTab } = $props();
</script>

{#if tab === 'containers' || report?.access !== 'Ok'}
  <DockerRows {sid} {report} />
{:else if tab === 'compose'}
  <DockerCompose {sid} />
{:else if tab === 'images'}
  <DockerImages {sid} />
{:else if tab === 'volumes'}
  <DockerVolumes {sid} />
{:else if tab === 'networks'}
  <DockerNetworks {sid} />
{:else}
  <DockerEvents {sid} />
{/if}
