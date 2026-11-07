<script lang="ts">
  import { Box } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import DockerRows from './DockerRows.svelte';
  import { services } from './services.svelte';

  /** Containers of the server (ADR-012): state, ports, load; logs and start / stop / restart. */
  let { sid }: { sid: string } = $props();

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
    <DockerRows {sid} {report} />
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
