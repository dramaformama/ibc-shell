<script lang="ts">
  import { Clock, FolderOpen, Gauge, SquareTerminal } from '@lucide/svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { connectHost } from '$lib/stores/actions';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { ago, percent, uptime } from './format';
  import { specText } from './spec-text';
  import Spark from './Spark.svelte';
  import { serverStatus } from './status';

  /** A quick look at a server from the overview list: system, recent load, three ways in. */
  let {
    server,
    rect,
    onenter,
    onleave
  }: { server: Server; rect: DOMRect; onenter: () => void; onleave: () => void } = $props();

  const sid = $derived(
    server.phase === 'connected' && server.group ? sessions.groupSession(server.group) : null
  );
  const live = $derived(stats.get(sid));
  const snap = $derived(lastSeen.get(server.host?.id));
  const s = $derived(live?.latest ? live : snap ? lastSeen.toSeries(snap) : null);
  const d = $derived(s?.latest ?? null);
  const st = $derived(
    serverStatus(server, settings.probe && reachability.data[server.id] === undefined)
  );
  const mem = $derived(d && d.mem_total ? (d.mem_used / d.mem_total) * 100 : null);
  // Below the row when it fits, above it near the bottom of the window.
  const below = $derived(rect.bottom + 250 < window.innerHeight);

  function open(kind: 'terminal' | 'sftp' | 'status') {
    onleave();
    if (server.host) connectHost(server.host.id, kind);
    else if (server.group) sessions.select(server.group.current.id);
  }
</script>

<div
  class="hc"
  role="dialog"
  aria-label={server.label}
  tabindex="-1"
  style:left="{Math.min(rect.left + 28, window.innerWidth - 340)}px"
  style:top={below ? `${rect.bottom + 4}px` : undefined}
  style:bottom={below ? undefined : `${window.innerHeight - rect.top + 4}px`}
  style:transform-origin={below ? 'top left' : 'bottom left'}
  onpointerenter={onenter}
  onpointerleave={onleave}
>
  <header>
    <Flag code={server.flag} size={28} />
    <div>
      <b>{server.label}</b>
      <small><i class="dot" data-phase={st.tone}></i>{st.text} · {server.address}</small>
    </div>
  </header>
  {#if d}
    <p class="sys">
      {d.os_name ?? ''}{#if d.uptime_s}<span
          ><Clock size={11} />{uptime(d.uptime_s, m.unit_d(), m.unit_h(), m.unit_m())}</span
        >{/if}
    </p>
    {#if d.cores}<p class="spec tnum">{specText(d)}</p>{/if}
    <div class="chart">
      <Spark
        series={[
          { values: s?.cpu ?? [], color: 'var(--accent)', fill: true },
          { values: s?.mem ?? [], color: 'var(--c-purple)' }
        ]}
        max={100}
        height={44}
      />
    </div>
    <p class="nums tnum">
      <span><i style:background="var(--accent)"></i>CPU <b>{percent(d.cpu_pct)}</b></span>
      <span><i style:background="var(--c-purple)"></i>RAM <b>{percent(mem)}</b></span>
      {#if !live?.latest && snap}<span class="when">{ago(snap.at, getLocale())}</span>{/if}
    </p>
  {:else}
    <p class="sys">{m.dash_metrics_hint()}</p>
  {/if}
  <footer>
    <button class="btn btn-sm btn-primary" onclick={() => open('terminal')}
      ><SquareTerminal size={13} />{m.tab_terminal()}</button
    >
    <button class="btn btn-sm" onclick={() => open('sftp')}
      ><FolderOpen size={13} />{m.tab_files()}</button
    >
    <button class="btn btn-sm" onclick={() => open('status')}
      ><Gauge size={13} />{m.tab_status()}</button
    >
  </footer>
</div>

<style>
  .hc {
    position: fixed;
    z-index: 45;
    width: 320px;
    padding: 14px;
    display: grid;
    gap: 8px;
    border: 0.5px solid var(--glass-edge);
    border-radius: var(--r-panel);
    background: var(--glass);
    -webkit-backdrop-filter: var(--glass-blur);
    backdrop-filter: var(--glass-blur);
    box-shadow: var(--shadow);
    animation: hc-in 320ms var(--spring);
    outline: none;
  }
  @keyframes hc-in {
    from {
      opacity: 0;
      transform: scale(0.94);
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  header div {
    display: grid;
    min-width: 0;
  }
  header b {
    font-size: var(--fs-head);
    font-weight: 600;
  }
  header small {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  p {
    margin: 0;
  }
  .sys {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    font-size: var(--fs-sm);
  }
  .sys span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }
  .spec {
    color: var(--muted);
    font-size: var(--fs-cap);
  }
  .chart {
    margin: 2px -2px 0;
  }
  .nums {
    display: flex;
    gap: 14px;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .nums i {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: 5px;
    border-radius: 50%;
    vertical-align: 1px;
  }
  .nums b {
    color: var(--text);
    font-weight: 600;
  }
  .nums .when {
    margin-left: auto;
  }
  footer {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
</style>
