<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { bytes } from '$lib/stats/format';
  import { dockerRes } from './dockerres.svelte';
  import ResGate from './ResGate.svelte';

  /** Images with size, age and use (ADR-016); unused ones are dimmed. */
  let { sid }: { sid: string } = $props();

  $effect(() => dockerRes.watch('images', sid));
  const report = $derived(dockerRes.images[sid] ?? null);
</script>

<ResGate
  access={report?.access ?? null}
  count={report?.images.length ?? 0}
  none={m.dk_images_none()}
>
  <ul class="res">
    {#each report?.images ?? [] as i (i.id + i.name)}
      <li class:unused={i.containers === 0}>
        <div class="who">
          <span class="name" title={i.name}>{i.name}</span>
          <span class="sub tnum">{i.id}</span>
        </div>
        <span class="mid sub">{i.created}</span>
        <span class="num">{i.size_bytes != null ? bytes(i.size_bytes) : '—'}</span>
        <span class="pill" class:off={i.containers === 0} style:--tone="var(--c-cyan)"
          >{i.containers ? m.dk_in_use({ count: i.containers }) : m.dk_unused()}</span
        >
      </li>
    {/each}
  </ul>
</ResGate>
