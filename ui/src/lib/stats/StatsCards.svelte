<script lang="ts">
  import {
    Activity,
    ArrowDown,
    ArrowUp,
    Cpu,
    HardDrive,
    ListOrdered,
    MemoryStick,
    Network
  } from '@lucide/svelte';
  import type { HostStats } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import type { Series } from '$lib/stores/stats.svelte';
  import CoreBars from './CoreBars.svelte';
  import { bytes, decimal, pct, rate } from './format';
  import Spark from './Spark.svelte';
  import './stats-cards.css';

  /** Metric cards shared by the side panel (narrow column) and the Status page (grid). */
  let { s, d, wide = false }: { s: Series; d: HostStats; wide?: boolean } = $props();

  const total = $derived(d.mem_total || 1);
  const cachedShown = $derived(Math.min(d.mem_cached, Math.max(0, d.mem_total - d.mem_used)));
  const freeShown = $derived(Math.max(0, d.mem_total - d.mem_used - cachedShown));
  const share = (v: number) => `${((v / total) * 100).toFixed(2)}%`;
  const diskPct = $derived(d.disk_total ? (d.disk_used / d.disk_total) * 100 : 0);
</script>

<div class="cards" class:wide>
  <section class="card cpu" style:--tone="var(--c-cyan)">
    <h3><Cpu size={15} />{m.stat_cpu_usage()}</h3>
    <div class="headline">
      <p class="big tnum">{pct(d.cpu_pct)}<small>%</small></p>
      {#if d.temp_c != null}<span class="temp tnum" data-hot={d.temp_c >= 75}
          >{Math.round(d.temp_c)}°C</span
        >{/if}
    </div>
    {#if wide}
      <Spark
        series={[{ values: s.cpu, color: 'var(--c-cyan)', fill: true }]}
        max={100}
        height={56}
      />
    {/if}
    <CoreBars cores={d.cpu_cores} />
    <dl class="legend tnum">
      <div>
        <dt>{m.stat_user()}</dt>
        <dd>{pct(d.cpu_user)}%</dd>
      </div>
      <div>
        <dt>{m.stat_system()}</dt>
        <dd>{pct(d.cpu_system)}%</dd>
      </div>
      <div>
        <dt>{m.stat_iowait()}</dt>
        <dd>{pct(d.cpu_iowait)}%</dd>
      </div>
      <div>
        <dt>{m.stat_steal()}</dt>
        <dd>{pct(d.cpu_steal)}%</dd>
      </div>
    </dl>
  </section>

  <section class="card" style:--tone="var(--c-green)">
    <h3><MemoryStick size={15} />{m.stat_memory()}</h3>
    <p class="big tnum">
      {pct(d.mem_total ? (d.mem_used / d.mem_total) * 100 : null)}<small>%</small>
      <span class="of">{bytes(d.mem_used)} / {bytes(d.mem_total)}</span>
    </p>
    <div class="stack" role="img" aria-label={m.stat_memory()}>
      <span style:width={share(d.mem_used)} style:background="var(--c-green)"></span>
      <span style:width={share(cachedShown)} style:background="var(--c-yellow)"></span>
    </div>
    <ul class="keys tnum">
      <li>
        <i style:background="var(--c-green)"></i>{m.stat_used_short()}<b>{bytes(d.mem_used)}</b>
      </li>
      <li>
        <i style:background="var(--c-yellow)"></i>{m.stat_cached()}<b>{bytes(cachedShown)}</b>
      </li>
      <li><i class="free"></i>{m.stat_free()}<b>{bytes(freeShown)}</b></li>
      {#if d.swap_total}
        <li>
          <i style:background="var(--c-purple)"></i>{m.stat_swap()}<b
            >{bytes(d.swap_used)} / {bytes(d.swap_total)}</b
          >
        </li>
      {/if}
    </ul>
  </section>

  <section class="card" style:--tone="var(--c-yellow)">
    <h3><Activity size={15} />{m.stat_load()}</h3>
    <Spark
      series={[
        { values: s.load15, color: 'var(--c-yellow)' },
        { values: s.load5, color: 'var(--c-cyan)' },
        { values: s.load1, color: 'var(--danger)', fill: true }
      ]}
      height={wide ? 72 : 56}
    />
    <div class="loads tnum">
      <span><i style:background="var(--danger)"></i>1 {m.unit_m()}<b>{decimal(d.load1)}</b></span>
      <span><i style:background="var(--c-cyan)"></i>5 {m.unit_m()}<b>{decimal(d.load5)}</b></span>
      <span
        ><i style:background="var(--c-yellow)"></i>15 {m.unit_m()}<b>{decimal(d.load15)}</b></span
      >
    </div>
  </section>

  <section class="card" style:--tone="var(--c-purple)">
    <h3><Network size={15} />{m.stat_network()}</h3>
    <Spark
      series={[
        { values: s.rx, color: 'var(--c-purple)', fill: true },
        { values: s.tx, color: 'var(--c-cyan)' }
      ]}
      height={wide ? 72 : 48}
    />
    <div class="io tnum">
      <span class="rx"
        ><ArrowDown size={13} /><b>{rate(d.net_rx_bps)[0]}</b>{rate(d.net_rx_bps)[1]}</span
      >
      <span class="tx"
        ><ArrowUp size={13} /><b>{rate(d.net_tx_bps)[0]}</b>{rate(d.net_tx_bps)[1]}</span
      >
    </div>
    {#if d.ifaces.length > 1 || wide}
      <ul class="ifaces tnum">
        {#each d.ifaces as i (i.name)}
          <li title={m.stat_since_boot({ rx: bytes(i.rx_total), tx: bytes(i.tx_total) })}>
            <span class="ifn mono">{i.name}</span>
            <span>↓ {rate(i.rx_bps).join(' ')}</span>
            <span>↑ {rate(i.tx_bps).join(' ')}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="card" style:--tone="var(--c-orange)">
    <h3><HardDrive size={15} />{m.stat_disk()}</h3>
    <p class="big tnum">
      {pct(d.disk_total ? diskPct : null)}<small>%</small>
      <span class="of">{bytes(d.disk_used)} / {bytes(d.disk_total)}</span>
    </p>
    <div class="meter" aria-hidden="true" data-hot={diskPct >= 90}>
      <span style:width="{diskPct}%"></span>
    </div>
    {#if wide}
      <Spark
        series={[
          { values: s.dr, color: 'var(--c-orange)', fill: true },
          { values: s.dw, color: 'var(--c-pink)' }
        ]}
        height={48}
      />
    {/if}
    <div class="io tnum">
      <span>{m.stat_read()} <b>{rate(d.disk_read_bps)[0]}</b>{rate(d.disk_read_bps)[1]}</span>
      <span>{m.stat_write()} <b>{rate(d.disk_write_bps)[0]}</b>{rate(d.disk_write_bps)[1]}</span>
    </div>
  </section>

  <section class="card procs-card" style:--tone="var(--c-pink)">
    <h3><ListOrdered size={15} />{m.stat_processes()}</h3>
    <table class="procs tnum">
      <thead>
        <tr>
          <th>PID</th><th>{m.stat_process()}</th>{#if wide}<th>{m.stat_owner()}</th>{/if}<th>CPU</th
          ><th>{m.stat_mem()}</th>
        </tr>
      </thead>
      <tbody>
        {#each d.procs as p (p.pid)}
          <tr>
            <td class="muted">{p.pid}</td>
            <td class="cmd" title="{p.command} · {p.user}">{p.command}</td>
            {#if wide}<td class="muted cmd">{p.user}</td>{/if}
            <td>{decimal(p.cpu, 1)}</td>
            <td>{bytes(p.mem_bytes)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
</div>
