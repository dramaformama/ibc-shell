<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    Clock,
    FolderOpen,
    Gauge,
    Pencil,
    Server as ServerIcon,
    SquareTerminal
  } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { connectHost } from '$lib/stores/actions';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { rate, uptime } from './format';
  import Ring from './Ring.svelte';
  import Spark from './Spark.svelte';

  /** Live card of a connected server (overview top section). */
  let { server }: { server: Server } = $props();
  const sid = $derived(server.group ? sessions.groupSession(server.group) : null);
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const live = $derived(stats.get(sid));
  $effect(() => {
    if (server.host && live?.latest && !live.stale) lastSeen.record(server.host.id, live);
  });
  const snap = $derived(lastSeen.get(server.host?.id));
  // Until the first live sample arrives, draw the remembered one (marked stale).
  const s = $derived(live?.latest ? live : snap ? lastSeen.toSeries(snap) : live);
  const d = $derived(s?.latest ?? null);

  function open(kind: 'terminal' | 'sftp' | 'status') {
    if (server.host) connectHost(server.host.id, kind);
    else if (server.group) sessions.select(server.group.current.id);
  }
</script>

<article class="card srv" aria-label={server.label}>
  <header>
    <button class="ident" onclick={() => open('status')} title={m.tab_status()}>
      <span class="server-icon"
        >{#if server.flag}<Flag code={server.flag} size={24} />{:else}<ServerIcon
            size={20}
          />{/if}</span
      >
      <span class="identity">
        <b>{server.label}</b>
        <small
          >{d?.os_name ?? server.address}{#if d}&nbsp;·&nbsp;<Clock size={10} />&nbsp;{uptime(
              d.uptime_s,
              m.unit_d(),
              m.unit_h(),
              m.unit_m()
            )}{/if}</small
        >
      </span>
    </button>
    <span
      class="live"
      class:stale={s?.stale}
      title={s?.stale ? m.stat_stale() : m.stat_live()}
      aria-label={s?.stale ? m.stat_stale() : m.stat_live()}
      ><i></i>{#if s?.stale}{m.stat_stale()}{/if}</span
    >
    {#if server.host}<button
        class="icon-btn edit"
        onclick={() => ui.openHostEditor(server.host!.id)}
        aria-label={m.dash_edit()}
        title={m.dash_edit()}><Pencil size={14} /></button
      >{/if}
  </header>

  {#if s?.unsupported}
    <p class="note">{m.stat_unsupported()}</p>
  {:else}
    <div class="rings">
      <div class="gauge">
        <Ring value={d?.cpu_pct ?? null} size={66} label={m.stat_cpu()} tone="var(--c-cyan)" />
        <span>{m.stat_cpu()}</span>
      </div>
      <div class="gauge">
        <Ring
          value={d && d.mem_total ? (d.mem_used / d.mem_total) * 100 : null}
          size={66}
          label={m.stat_ram()}
          tone="var(--c-green)"
        />
        <span>{m.stat_ram()}</span>
      </div>
      <div class="gauge">
        <Ring
          value={d && d.disk_total ? (d.disk_used / d.disk_total) * 100 : null}
          size={66}
          label={m.stat_disk()}
          tone="var(--c-orange)"
        />
        <span>{m.stat_disk()}</span>
      </div>
    </div>
    <div class="net">
      <Spark
        series={[
          { values: s?.rx ?? [], color: 'var(--c-purple)', fill: true },
          { values: s?.tx ?? [], color: 'var(--c-cyan)' }
        ]}
        height={34}
      />
      <div class="io tnum">
        <span class="rx"
          ><ArrowDown size={12} /><b>{d ? rate(d.net_rx_bps)[0] : '—'}</b>{d
            ? rate(d.net_rx_bps)[1]
            : ''}</span
        >
        <span class="tx"
          ><ArrowUp size={12} /><b>{d ? rate(d.net_tx_bps)[0] : '—'}</b>{d
            ? rate(d.net_tx_bps)[1]
            : ''}</span
        >
      </div>
    </div>
  {/if}

  <footer>
    <button class="btn btn-sm btn-tonal" onclick={() => open('terminal')}
      ><SquareTerminal size={14} />{m.tab_terminal()}</button
    >
    <button class="btn btn-sm btn-ghost" onclick={() => open('sftp')} disabled={!server.host}
      ><FolderOpen size={14} />{m.tab_files()}</button
    >
    <button class="btn btn-sm btn-ghost" onclick={() => open('status')} disabled={!server.host}
      ><Gauge size={14} />{m.tab_status()}</button
    >
  </footer>
</article>

<style>
  .srv {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 16px 0;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ident {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .server-icon {
    display: grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 13px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .identity {
    display: grid;
    min-width: 0;
  }
  .identity b {
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.015em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .identity small {
    display: flex;
    align-items: center;
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    white-space: nowrap;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--ok);
    font-size: 10px;
    font-weight: 600;
    white-space: nowrap;
  }
  .live i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 0 3px color-mix(in oklab, currentColor 18%, transparent);
  }
  .live.stale {
    color: var(--warn);
  }
  .edit {
    width: 28px;
    height: 28px;
  }
  .rings {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .gauge {
    display: grid;
    justify-items: center;
    gap: 5px;
  }
  .gauge > span {
    font-size: 11px;
    color: var(--muted);
  }
  .net {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 14px;
    background: var(--surface-2);
  }
  .io {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--muted);
  }
  .io span {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }
  .io b {
    color: var(--text);
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .rx :global(svg) {
    color: var(--c-purple);
    align-self: center;
  }
  .tx :global(svg) {
    color: var(--c-cyan);
    align-self: center;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: auto -16px 0;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }
</style>
