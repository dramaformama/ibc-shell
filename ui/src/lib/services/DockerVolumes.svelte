<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { bytes } from '$lib/stats/format';
  import { dockerRes } from './dockerres.svelte';
  import ResGate from './ResGate.svelte';

  /** Volumes with disk use and the containers mounting them (ADR-016). */
  let { sid }: { sid: string } = $props();

  $effect(() => dockerRes.watch('volumes', sid));
  const report = $derived(dockerRes.volumes[sid] ?? null);
</script>

<ResGate
  access={report?.access ?? null}
  count={report?.volumes.length ?? 0}
  none={m.dk_volumes_none()}
>
  <ul class="res">
    {#each report?.volumes ?? [] as v (v.name)}
      <li class:unused={v.containers === 0}>
        <div class="who">
          <span class="name" title={v.name}>{v.name}</span>
          <span class="sub">{v.driver}</span>
        </div>
        <span class="mid sub"></span>
        <span class="num">{v.size_bytes != null ? bytes(v.size_bytes) : '—'}</span>
        <span class="pill" class:off={v.containers === 0} style:--tone="var(--c-cyan)"
          >{v.containers ? m.dk_in_use({ count: v.containers }) : m.dk_unused()}</span
        >
      </li>
    {/each}
  </ul>
</ResGate>
