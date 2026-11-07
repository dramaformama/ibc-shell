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

<style>
  .cards {
    display: grid;
    gap: 14px;
    align-content: start;
  }
  /* Masonry: cards keep their natural height instead of stretching to the row. */
  .cards.wide {
    display: block;
    columns: 3 300px;
    column-gap: 16px;
  }
  .cards.wide > .card {
    break-inside: avoid;
    margin-bottom: 16px;
  }
  .cards.wide .procs-card {
    column-span: all;
  }
  .card {
    display: grid;
    gap: 10px;
    align-content: start;
    padding: 16px 18px;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  h3 :global(svg) {
    width: 28px;
    height: 28px;
    padding: 6px;
    border-radius: 9px;
    background: color-mix(in oklab, var(--tone) 13%, transparent);
    color: var(--tone);
  }
  .headline {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .big {
    margin: 0;
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
  }
  .big small {
    margin-left: 2px;
    font-size: 15px;
    color: var(--muted);
    font-weight: 500;
  }
  .of {
    margin-left: 8px;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
    color: var(--muted);
  }
  .temp {
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .temp[data-hot='true'] {
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .legend {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    margin: 0;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    font-size: 11px;
  }
  .legend dt {
    color: var(--muted);
  }
  .legend dd {
    margin: 0;
    font-weight: 600;
    font-size: 12px;
  }
  .stack {
    display: flex;
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-2);
  }
  .stack span {
    height: 100%;
    transition: width 600ms var(--ease);
  }
  .stack span + span {
    box-shadow: inset 1.5px 0 0 var(--surface);
  }
  .keys {
    display: grid;
    gap: 5px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--muted);
  }
  .keys li,
  .loads span {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .keys b,
  .loads b {
    margin-left: auto;
    color: var(--text);
    font-weight: 600;
  }
  .keys i,
  .loads i {
    width: 8px;
    height: 8px;
    border-radius: 3px;
    flex: none;
  }
  .keys i.free {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .loads {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    font-size: 12px;
    color: var(--muted);
  }
  .loads b {
    margin-left: 2px;
  }
  .io {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--muted);
  }
  .io span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .io b {
    color: var(--text);
    font-size: 14px;
    font-weight: 700;
  }
  .io .rx :global(svg) {
    color: var(--c-purple);
  }
  .io .tx :global(svg) {
    color: var(--c-cyan);
  }
  .ifaces {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 8px 0 0;
    border-top: 1px solid var(--border);
    list-style: none;
    font-size: 11px;
    color: var(--muted);
  }
  .ifaces li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 10px;
  }
  .ifn {
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meter {
    height: 10px;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .meter span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--tone);
    transition: width 600ms var(--ease);
  }
  .meter[data-hot='true'] span {
    background: var(--danger);
  }
  .procs {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    table-layout: fixed;
  }
  .procs th {
    padding: 0 0 6px;
    color: var(--tone);
    font-weight: 600;
    text-align: left;
  }
  .procs th:nth-child(1) {
    width: 56px;
  }
  .procs th:nth-last-child(1),
  .procs th:nth-last-child(2) {
    width: 56px;
    text-align: right;
  }
  .procs td {
    padding: 5px 0;
    border-top: 1px solid var(--border);
  }
  .procs td:nth-last-child(1),
  .procs td:nth-last-child(2) {
    text-align: right;
  }
  .cmd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-right: 8px !important;
  }
  .muted {
    color: var(--muted);
  }
</style>
