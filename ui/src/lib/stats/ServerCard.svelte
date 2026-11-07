<script lang="ts">
  import { ArrowDown, ArrowUp, Clock, Cpu, HardDrive, MemoryStick } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { bytes, rate, uptime } from './format';
  import Ring from './Ring.svelte';

  /** Dashboard card for one connected server (SwiftServer-style). */
  let { group }: { group: TabGroup } = $props();

  const sid = $derived(sessions.groupSession(group));
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const d = $derived(stats.get(sid)?.latest ?? null);
  const phase = $derived(sessions.groupPhase(group));
</script>

<button class="card srv" onclick={() => sessions.select(group.current.id)}>
  <div class="top">
    <Flag code={group.flag} size={20} />
    <span class="name">{group.label}</span>
    <span class="dot" data-phase={phase}></span>
  </div>
  {#if d}
    <div class="facts tnum">
      <span><Cpu size={12} />{d.cores}</span>
      <span><MemoryStick size={12} />{bytes(d.mem_total)}</span>
      <span><HardDrive size={12} />{bytes(d.disk_total)}</span>
      <span><Clock size={12} />{uptime(d.uptime_s, m.unit_d(), m.unit_h(), m.unit_m())}</span>
    </div>
    <div class="body">
      <div class="g"><span class="cap">{m.stat_cpu()}</span><Ring value={d.cpu_pct} label={m.stat_cpu()} /></div>
      <div class="g">
        <span class="cap">{m.stat_ram()}</span>
        <Ring value={d.mem_total ? (d.mem_used / d.mem_total) * 100 : 0} label={m.stat_ram()} />
      </div>
      <div class="g io tnum">
        <span class="cap">{m.stat_network()}</span>
        <span><ArrowUp size={12} /><b>{rate(d.net_tx_bps)[0]}</b>{rate(d.net_tx_bps)[1]}</span>
        <span><ArrowDown size={12} /><b>{rate(d.net_rx_bps)[0]}</b>{rate(d.net_rx_bps)[1]}</span>
      </div>
    </div>
  {:else}
    <p class="wait">{sid ? m.stat_waiting() : m.stat_no_session()}</p>
  {/if}
</button>

<style>
  .srv {
    display: grid;
    gap: 12px;
    padding: 16px 18px;
    border: 0;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      transform 160ms var(--ease),
      box-shadow 160ms var(--ease);
  }
  .srv:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
  }
  .facts span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .body {
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 18px;
    align-items: center;
  }
  .g {
    display: grid;
    justify-items: center;
    gap: 6px;
  }
  .cap {
    color: var(--muted);
    font-size: 12px;
  }
  .io {
    justify-items: start;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .io span:not(.cap) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .io b {
    color: var(--text);
    font-size: 15px;
    font-weight: 700;
  }
  .wait {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
</style>
