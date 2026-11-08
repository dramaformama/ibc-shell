<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { dockerRes } from './dockerres.svelte';
  import ResGate from './ResGate.svelte';

  /** Networks with driver, subnet and attached containers (ADR-016). */
  let { sid }: { sid: string } = $props();

  $effect(() => dockerRes.watch('networks', sid));
  const report = $derived(dockerRes.networks[sid] ?? null);
</script>

<ResGate
  access={report?.access ?? null}
  count={report?.networks.length ?? 0}
  none={m.dk_networks_none()}
>
  <ul class="res">
    {#each report?.networks ?? [] as n (n.id)}
      <li class:unused={n.containers === 0}>
        <div class="who">
          <span class="name" title={n.name}>{n.name}</span>
          <span class="sub">{n.driver} · {n.scope}</span>
        </div>
        <span class="mid sub tnum" title={n.subnet}>{n.subnet || '—'}</span>
        <span class="num"></span>
        <span class="pill" class:off={n.containers === 0} style:--tone="var(--c-cyan)"
          >{n.containers ? m.dk_in_use({ count: n.containers }) : m.dk_unused()}</span
        >
      </li>
    {/each}
  </ul>
</ResGate>
