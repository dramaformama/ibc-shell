<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    Clock,
    Cpu,
    FolderOpen,
    HardDrive,
    MemoryStick,
    Pencil,
    Server as ServerIcon,
    SquareTerminal
  } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { bytes, rate, uptime } from './format';
  import Ring from './Ring.svelte';

  let { server }: { server: Server } = $props();
  const sid = $derived(server.group ? sessions.groupSession(server.group) : null);
  $effect(() => {
    if (server.host && server.phase !== 'connected') return reachability.watch(server.host.id);
  });
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const series = $derived(stats.get(sid));
  const d = $derived(series?.latest ?? null);
  const status = $derived(
    server.phase === 'connecting'
      ? m.dash_connecting()
      : server.phase === 'failed'
        ? m.dash_connection_error()
        : server.phase === 'connected'
          ? m.status_connected()
          : server.availability === 'online'
            ? m.dash_available()
            : server.availability === 'offline'
              ? m.dash_unavailable()
              : reachability.data[server.id] === undefined
                ? m.dash_checking()
                : m.dash_not_checked()
  );
  const tone = $derived(
    server.phase === 'failed'
      ? 'failed'
      : server.phase === 'connecting'
        ? 'connecting'
        : server.availability === 'online'
          ? 'connected'
          : server.availability === 'offline'
            ? 'offline'
            : 'unknown'
  );

  function open(kind: 'terminal' | 'sftp') {
    if (server.host) connectHost(server.host.id, kind);
    else if (server.group) sessions.select(server.group.current.id);
  }
</script>

<article class="card srv" aria-label={server.label}>
  <header>
    <span class="server-icon"
      >{#if server.flag}<Flag code={server.flag} size={24} />{:else}<ServerIcon
          size={22}
        />{/if}</span
    >
    <div class="identity">
      <h3 title={server.label}>{server.label}</h3>
      <p title={server.address}>{server.address}</p>
    </div>
    {#if server.host}<button
        class="icon-btn edit"
        onclick={() => ui.openHostEditor(server.host!.id)}
        aria-label={m.dash_edit()}
        title={m.dash_edit()}><Pencil size={14} /></button
      >{/if}
  </header>
  <div class="status-row">
    <span class="status" data-phase={tone} title={m.dash_port_hint()}
      ><i class="dot" data-phase={tone}></i>{status}</span
    >
    <span class="location">{server.folder}</span>
    {#if server.latency != null}<span
        class="latency tnum"
        title={m.dash_latency({ ms: server.latency })}>{server.latency} ms</span
      >{/if}
  </div>
  <div class="metrics" class:unavailable={!d}>
    <div class="gauge">
      <Ring value={d?.cpu_pct ?? null} size={64} label={m.stat_cpu()} tone="var(--accent)" /><span
        >{m.stat_cpu()}</span
      >
    </div>
    <div class="gauge">
      <Ring
        value={d && d.mem_total ? (d.mem_used / d.mem_total) * 100 : null}
        size={64}
        label={m.stat_ram()}
        tone="var(--c-purple)"
      /><span>{m.stat_ram()}</span>
    </div>
    <div class="throughput tnum">
      <span class="cap">{m.stat_network()}</span>
      <span
        ><ArrowDown size={13} /><b>{d ? rate(d.net_rx_bps)[0] : '—'}</b><small
          >{d ? rate(d.net_rx_bps)[1] : ''}</small
        ></span
      >
      <span
        ><ArrowUp size={13} /><b>{d ? rate(d.net_tx_bps)[0] : '—'}</b><small
          >{d ? rate(d.net_tx_bps)[1] : ''}</small
        ></span
      >
    </div>
  </div>
  <div class="facts tnum">
    {#if d}
      <span title={m.stat_cores()}><Cpu size={12} />{d.cores}</span><span
        ><MemoryStick size={12} />{bytes(d.mem_total)}</span
      ><span><HardDrive size={12} />{bytes(d.disk_total)}</span><span title={m.stat_uptime()}
        ><Clock size={12} />{uptime(d.uptime_s, m.unit_d(), m.unit_h(), m.unit_m())}</span
      >
    {:else}
      <span class="explanation"
        >{series?.unsupported
          ? m.stat_unsupported()
          : sid
            ? m.stat_waiting()
            : server.availability === 'offline'
              ? m.dash_offline_hint()
              : m.dash_metrics_hint()}</span
      >
    {/if}
  </div>
  <footer>
    <button class="btn btn-sm" class:btn-tonal={!server.group} onclick={() => open('terminal')}
      ><SquareTerminal size={14} />{server.group ? m.tab_terminal() : m.dash_connect()}</button
    >
    <button class="btn btn-sm btn-ghost" onclick={() => open('sftp')} disabled={!server.host}
      ><FolderOpen size={14} />{m.tab_files()}</button
    >
    {#if d}<span
        class="freshness"
        class:stale={series?.stale}
        title={series?.updatedAt
          ? m.stat_sampled({ time: new Date(series.updatedAt).toLocaleTimeString() })
          : ''}>{series?.stale ? m.stat_stale() : m.stat_live()}</span
      >{/if}
  </footer>
</article>

<style>
  .srv {
    height: 288px;
    display: flex;
    flex-direction: column;
    padding: 18px 18px 0;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .server-icon {
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .identity {
    flex: 1;
    min-width: 0;
  }
  h3 {
    margin: 0 0 2px;
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.015em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .identity p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .edit {
    width: 28px;
    height: 28px;
  }
  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    min-width: 0;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
  }
  .status[data-phase='connected'] {
    color: var(--ok);
  }
  .status[data-phase='failed'],
  .status[data-phase='offline'] {
    color: var(--danger);
  }
  .status[data-phase='connecting'] {
    color: var(--warn);
  }
  .dot[data-phase='offline'] {
    background: var(--danger);
  }
  .location {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--muted);
  }
  .latency {
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .metrics {
    display: grid;
    grid-template-columns: 1fr 1fr 1.25fr;
    gap: 12px;
    align-items: center;
    padding: 10px 0 12px;
    border-top: 1px solid var(--border);
  }
  .gauge {
    display: grid;
    justify-items: center;
    gap: 5px;
  }
  .gauge > span,
  .cap {
    font-size: 11px;
    color: var(--muted);
  }
  .throughput {
    display: grid;
    gap: 5px;
  }
  .throughput > span:not(.cap) {
    display: flex;
    align-items: baseline;
    gap: 4px;
    color: var(--muted);
  }
  .throughput b {
    color: var(--text);
    font-size: 17px;
    line-height: 1.2;
    font-weight: 600;
    letter-spacing: -0.025em;
  }
  .throughput small {
    font-size: 11px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    height: 33px;
    color: var(--muted);
    font-size: 11px;
  }
  .facts span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .explanation {
    line-height: 1.35;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 4px;
    border-top: 1px solid var(--border);
    margin: auto -18px 0;
    padding: 10px 14px;
    background: color-mix(in oklab, var(--surface-2) 30%, transparent);
  }
  .freshness {
    margin-left: auto;
    color: var(--ok);
    font-size: 10px;
    max-width: 85px;
    text-align: right;
    line-height: 1.2;
  }
  .stale {
    color: var(--warn);
  }
</style>
