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
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import CoreBars from './CoreBars.svelte';
  import { bytes, decimal, pct, rate, uptime } from './format';
  import Ring from './Ring.svelte';
  import Spark from './Spark.svelte';

  /** SwiftServer-style status column next to the shell. */
  let { group }: { group: TabGroup } = $props();

  const sid = $derived(sessions.groupSession(group));
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const s = $derived(stats.get(sid));
  const d = $derived(s?.latest ?? null);
  const up = (v: number) => uptime(v, m.unit_d(), m.unit_h(), m.unit_m());
</script>

<aside class="insp" aria-label={m.stat_inspector()}>
  {#if !sid}
    <p class="hint">{m.stat_no_session()}</p>
  {:else if s?.unsupported}
    <p class="hint">{m.stat_unsupported()}</p>
  {:else if !d}
    <p class="hint"><span class="dot" data-phase="connecting"></span>{m.stat_waiting()}</p>
  {:else}
    <header>
      <div class="inspector-title">
        <h2>{m.stat_inspector()}</h2>
        <span class:stale={s?.stale}>{s?.stale ? m.stat_stale() : m.stat_live()}</span>
      </div>
      {#if d.os_name}<span class="os">{d.os_name}</span>{/if}
      <div class="facts tnum">
        <span>{d.cores} {m.stat_cores().toLowerCase()}</span>
        <span>{bytes(d.mem_total)}</span>
        <span>{m.stat_uptime()} {up(d.uptime_s)}</span>
        {#if d.temp_c != null}<span>{Math.round(d.temp_c)}°C</span>{/if}
      </div>
    </header>

    <section class="card" style:--tone="var(--c-cyan)">
      <h3><Cpu size={15} />{m.stat_cpu_usage()}</h3>
      <p class="big tnum">{pct(d.cpu_pct)}<small>%</small></p>
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

    <section class="card" style:--tone="var(--c-yellow)">
      <h3><Activity size={15} />{m.stat_load()}</h3>
      <Spark
        series={[
          { values: s?.load15 ?? [], color: 'var(--c-yellow)' },
          { values: s?.load5 ?? [], color: 'var(--c-cyan)' },
          { values: s?.load1 ?? [], color: 'var(--danger)', fill: true }
        ]}
      />
      <div class="kv tnum">
        <span>{m.stat_current_load()}</span>
        <b>{decimal(d.load1)} / {decimal(d.load5)} / {decimal(d.load15)}</b>
      </div>
    </section>

    <section class="card" style:--tone="var(--c-green)">
      <h3><MemoryStick size={15} />{m.stat_memory()}</h3>
      <div class="mem">
        <Ring
          value={d.mem_total ? (d.mem_used / d.mem_total) * 100 : 0}
          label={m.stat_ram()}
          tone="var(--c-green)"
        />
        <div class="memtext tnum">
          <b>{m.stat_used({ used: bytes(d.mem_used), total: bytes(d.mem_total) })}</b>
          <span>{m.stat_cached()} {bytes(d.mem_cached)}</span>
          {#if d.swap_total}<span
              >{m.stat_swap()}
              {m.stat_used({ used: bytes(d.swap_used), total: bytes(d.swap_total) })}</span
            >{/if}
        </div>
      </div>
    </section>

    <section class="card" style:--tone="var(--c-purple)">
      <h3><Network size={15} />{m.stat_network()}</h3>
      <Spark
        series={[
          { values: s?.rx ?? [], color: 'var(--c-purple)', fill: true },
          { values: s?.tx ?? [], color: 'var(--c-cyan)' }
        ]}
        height={48}
      />
      <div class="io tnum">
        <span><ArrowDown size={13} /><b>{rate(d.net_rx_bps)[0]}</b>{rate(d.net_rx_bps)[1]}</span>
        <span><ArrowUp size={13} /><b>{rate(d.net_tx_bps)[0]}</b>{rate(d.net_tx_bps)[1]}</span>
      </div>
    </section>

    <section class="card" style:--tone="var(--c-orange)">
      <h3><HardDrive size={15} />{m.stat_disk()}</h3>
      <div class="meter" aria-hidden="true">
        <span style:width="{d.disk_total ? (d.disk_used / d.disk_total) * 100 : 0}%"></span>
      </div>
      <div class="kv tnum">
        <span>/</span>
        <b>{m.stat_used({ used: bytes(d.disk_used), total: bytes(d.disk_total) })}</b>
      </div>
      <div class="io tnum">
        <span>{m.stat_read()} <b>{rate(d.disk_read_bps)[0]}</b>{rate(d.disk_read_bps)[1]}</span>
        <span>{m.stat_write()} <b>{rate(d.disk_write_bps)[0]}</b>{rate(d.disk_write_bps)[1]}</span>
      </div>
    </section>

    <section class="card" style:--tone="var(--c-pink)">
      <h3><ListOrdered size={15} />{m.stat_processes()}</h3>
      <table class="procs tnum">
        <thead>
          <tr><th>PID</th><th>{m.stat_process()}</th><th>CPU</th><th>{m.stat_mem()}</th></tr>
        </thead>
        <tbody>
          {#each d.procs as p (p.pid)}
            <tr>
              <td class="muted">{p.pid}</td>
              <td class="cmd" title="{p.command} · {p.user}">{p.command}</td>
              <td>{decimal(p.cpu, 1)}</td>
              <td>{bytes(p.mem_bytes)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}
</aside>

<style>
  .insp {
    width: 340px;
    flex: none;
    overflow-y: auto;
    padding: 16px;
    display: grid;
    gap: 16px;
    align-content: start;
    border-left: 1px solid var(--border);
    background: var(--bg);
  }
  .inspector-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }
  .inspector-title h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }
  .inspector-title span {
    font-size: 10px;
    color: var(--ok);
  }
  .inspector-title .stale {
    color: var(--warn);
  }
  h3 :global(svg) {
    width: 26px;
    height: 26px;
    padding: 5px;
    border-radius: 7px;
    background: color-mix(in oklab, var(--tone) 12%, transparent);
    color: var(--tone);
  }
  @media (max-width: 1000px) {
    .insp {
      position: absolute;
      z-index: 8;
      inset: 0 0 0 auto;
      width: min(340px, 86%);
      box-shadow: var(--shadow);
    }
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 16px 8px;
    color: var(--muted);
    font-size: 12px;
  }
  header {
    display: grid;
    gap: 8px;
    padding: 4px 4px 0;
  }
  .os {
    justify-self: start;
    padding: 3px 10px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--c-purple) 14%, transparent);
    color: var(--c-purple);
    font-size: 12px;
    font-weight: 600;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    color: var(--muted);
    font-size: 12px;
  }
  .card {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  .big {
    margin: -2px 0 0;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
  }
  .big small {
    margin-left: 2px;
    font-size: 16px;
    color: var(--muted);
    font-weight: 500;
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
  .kv {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .kv b {
    color: var(--text);
    font-weight: 600;
  }
  .mem {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .memtext {
    display: grid;
    gap: 3px;
    font-size: 12px;
    color: var(--muted);
  }
  .memtext b {
    color: var(--text);
    font-size: 13px;
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
  .meter {
    height: 8px;
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
    width: 52px;
  }
  .procs th:nth-child(3),
  .procs th:nth-child(4) {
    width: 50px;
    text-align: right;
  }
  .procs td {
    padding: 4px 0;
    border-top: 1px solid var(--border);
  }
  .procs td:nth-child(3),
  .procs td:nth-child(4) {
    text-align: right;
  }
  .cmd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-right: 6px !important;
  }
  .muted {
    color: var(--muted);
  }
</style>
