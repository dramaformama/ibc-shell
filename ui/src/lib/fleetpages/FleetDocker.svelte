<script lang="ts">
  import { Box } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import DockerRows from '$lib/services/DockerRows.svelte';
  import { services } from '$lib/services/services.svelte';
  import { connectedServers } from './connected.svelte';
  import './page.css';

  /** Containers of every connected server on one page (ADR-012). */
  const live = $derived(connectedServers());
  $effect(() => {
    const stops = live.map((s) => services.watch('docker', s.sid));
    return () => stops.forEach((stop) => stop());
  });
  const all = $derived(live.flatMap((s) => services.docker[s.sid]?.containers ?? []));
  const running = $derived(all.filter((c) => c.state === 'running').length);
  // Servers without Docker are listed last, in one line.
  const withDocker = $derived(live.filter((s) => services.docker[s.sid]?.access !== 'Missing'));
  const without = $derived(live.filter((s) => services.docker[s.sid]?.access === 'Missing'));
</script>

<section class="fpage" aria-label={m.dk_title()}>
  <header>
    <h1>{m.dk_title()}</h1>
    <p class="subtitle tnum">
      {m.fleet_dk_counts({ servers: live.length, running, total: all.length })}
    </p>
  </header>
  {#if !live.length}
    <div class="blank">
      <Box size={28} />
      <p>{m.fleet_need_connect()}</p>
    </div>
  {:else}
    {#each withDocker as s (s.key)}
      <section class="card server" style:--tone="var(--c-cyan)">
        <h2><Flag code={s.group.flag} size={18} />{s.group.label}</h2>
        <DockerRows sid={s.sid} report={services.docker[s.sid] ?? null} />
      </section>
    {/each}
    {#if without.length}
      <p class="subtitle">
        {m.fleet_dk_missing({ names: without.map((s) => s.group.label).join(', ') })}
      </p>
    {/if}
  {/if}
</section>
