<script lang="ts">
  import { scrollEdge } from '$lib/util/scroll-edge.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { Box } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import '$lib/services/docker-res.css';
  import DockerTabs from '$lib/services/DockerTabs.svelte';
  import { DOCKER_TABS, type DockerTab } from '$lib/services/docker-tabs';
  import { services } from '$lib/services/services.svelte';
  import { connectedServers } from './connected.svelte';
  import './page.css';

  /** Everything Docker shows, for every connected server on one page (ADR-012, ADR-016). */
  let tab = $state<DockerTab>('containers');
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

<section class="fpage" aria-label={m.dk_title()} use:scrollEdge>
  <header>
    <h1>{m.dk_title()}</h1>
    <p class="subtitle tnum">
      {m.fleet_dk_counts({ servers: live.length, running, total: all.length })}
    </p>
  </header>
  {#if !live.length}
    <EmptyState icon={Box} title={m.fleet_need_connect_title()} hint={m.fleet_need_connect_hint()}>
      <button
        class="btn btn-primary btn-sm"
        onclick={() => {
          ui.page = 'overview';
          sessions.activeId = null;
        }}>{m.fleet_open_overview()}</button
      >
    </EmptyState>
  {:else}
    <div class="seg tabs" role="tablist" aria-label={m.dk_title()}>
      {#each DOCKER_TABS as [id, label] (id)}
        <button type="button" role="tab" aria-selected={tab === id} onclick={() => (tab = id)}
          >{label()}</button
        >
      {/each}
    </div>
    {#each withDocker as s (s.key)}
      <section class="card server dk" style:--tone="var(--c-cyan)">
        <h2><Flag code={s.group.flag} size={18} />{s.group.label}</h2>
        <DockerTabs sid={s.sid} report={services.docker[s.sid] ?? null} {tab} />
      </section>
    {/each}
    {#if without.length}
      <p class="subtitle">
        {m.fleet_dk_missing({ names: without.map((s) => s.group.label).join(', ') })}
      </p>
    {/if}
  {/if}
</section>
