<script lang="ts">
  import './docker-res.css';
  import { Box } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import DockerTabs from './DockerTabs.svelte';
  import { DOCKER_TABS as tabs, type DockerTab } from './docker-tabs';
  import { services } from './services.svelte';

  /** Docker of the server (ADR-012, ADR-016): containers, compose, images, volumes, networks, events. */
  let { sid }: { sid: string } = $props();

  let tab = $state<DockerTab>('containers');

  $effect(() => services.watch('docker', sid));
  const report = $derived(services.docker[sid] ?? null);
  const running = $derived(report?.containers.filter((c) => c.state === 'running').length ?? 0);
  const stopped = $derived((report?.containers.length ?? 0) - running);
</script>

{#if report?.access !== 'Missing'}
  <section class="card dk" style:--tone="var(--c-cyan)">
    <h3>
      <Box size={15} />{m.dk_title()}
      {#if report?.access === 'Ok' && report.containers.length}
        <span class="count tnum"
          >{m.dk_running({ count: running })}{#if stopped}<span class="sep">·</span>{m.dk_stopped({
              count: stopped
            })}{/if}</span
        >
      {/if}
    </h3>
    {#if report?.access === 'Ok'}
      <div class="seg tabs" role="tablist" aria-label={m.dk_title()}>
        {#each tabs as [id, label] (id)}
          <button type="button" role="tab" aria-selected={tab === id} onclick={() => (tab = id)}
            >{label()}</button
          >
        {/each}
      </div>
    {/if}
    <DockerTabs {sid} {report} {tab} />
  </section>
{/if}

<style>
  .count {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
  }
  .sep {
    margin: 0 0.35em;
  }
</style>
