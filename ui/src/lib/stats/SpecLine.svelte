<script lang="ts">
  import { Cpu, HardDrive, MemoryStick } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { cpuShort, type SpecInput } from './spec';
  import { coresText, diskText, sizeText } from './spec-text';

  /** What the server is: cores, memory, disk (and the processor name when `model`). */
  let {
    d,
    model = false,
    inline = false
  }: { d: SpecInput; model?: boolean; inline?: boolean } = $props();

  const disk = $derived(diskText(d));
  const cpu = $derived(d.cpu_model || null);
</script>

<div class="spec tnum" class:inline>
  <span class="chips">
    {#if d.cores}<span title={cpu ?? m.spec_cpu()}><Cpu size={13} />{coresText(d.cores)}</span>{/if}
    {#if d.mem_total}<span title={m.stat_ram()}
        ><MemoryStick size={13} />{sizeText(d.mem_total)}</span
      >{/if}
    {#if disk}<span title={m.stat_disk()}><HardDrive size={13} />{disk}</span>{/if}
  </span>
  {#if model && cpu}<span class="model" title={cpu}>{cpuShort(cpu)}</span>{/if}
</div>

<style>
  .spec {
    display: grid;
    gap: 3px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  /* `inline`: the chips join the parent's flex row instead of forming their own. */
  .spec.inline,
  .spec.inline .chips {
    display: contents;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
  }
  .chips > span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }
  .model {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    opacity: 0.85;
  }
</style>
