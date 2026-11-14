<script lang="ts">
  import { FolderOpen, Gauge, Pencil, Server as ServerIcon, Trash2 } from '@lucide/svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { hkey } from '$lib/util/tree-model';
  import { ago, ms, percent } from './format';
  import Roll from './Roll.svelte';
  import { serverStatus } from './status';

  /** One server in the overview list: state, identity, load, ping, quick actions. */
  let { server }: { server: Server } = $props();

  $effect(() => {
    if (settings.probe && server.host && server.phase !== 'connected')
      return reachability.watch(server.host.id);
  });
  // A connected server streams live metrics; they also refresh the remembered snapshot.
  const sid = $derived(
    server.phase === 'connected' && server.group ? sessions.groupSession(server.group) : null
  );
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const live = $derived(stats.get(sid));
  $effect(() => {
    if (server.host && live?.latest && !live.stale) lastSeen.record(server.host.id, live);
  });
  const snap = $derived(lastSeen.get(server.host?.id));
  const d = $derived(live?.latest && !live.stale ? live.latest : (snap?.d ?? null));
  const fresh = $derived(!!(live?.latest && !live.stale));
  const mem = $derived(d && d.mem_total ? (d.mem_used / d.mem_total) * 100 : null);
  const disk = $derived(d && d.disk_total ? (d.disk_used / d.disk_total) * 100 : null);
  const st = $derived(
    serverStatus(server, settings.probe && reachability.data[server.id] === undefined)
  );
  const hot = (v: number | null) => (v == null ? '' : v >= 90 ? 'hot' : v >= 70 ? 'warm' : '');

  function open(kind: 'terminal' | 'sftp' | 'status') {
    if (server.host) connectHost(server.host.id, kind);
    else if (server.group) sessions.select(server.group.current.id);
  }
</script>

<div class="row" role="row" aria-label={server.label}>
  <button
    class="hit"
    onclick={() => open('terminal')}
    title={m.dash_open_terminal()}
    aria-label="{server.label}, {st.text}"
  ></button>
  <span class="c-name" role="cell">
    <i class="dot" data-phase={st.tone} aria-hidden="true"></i>
    {#if server.flag}<Flag code={server.flag} size={18} />{:else}<span class="noflag"
        ><ServerIcon size={13} /></span
      >{/if}
    <b>{server.label}</b>
  </span>
  <span class="c-addr tnum" role="cell">{server.address}</span>
  <span class="c-folder" role="cell">{server.folder}</span>
  <span
    class="c-load tnum"
    class:stale={!fresh}
    role="cell"
    title={snap && !fresh ? m.stat_last({ when: ago(snap.at, getLocale()) }) : undefined}
  >
    {#if d}
      <span class={hot(d.cpu_pct)}><Roll value={percent(d.cpu_pct)} /></span><span class={hot(mem)}
        ><Roll value={percent(mem)} /></span
      ><span class={hot(disk)}><Roll value={percent(disk)} /></span>
    {/if}
  </span>
  <span class="c-ping tnum" role="cell"
    >{#if server.latency != null}<Roll value={ms(server.latency)} />{/if}</span
  >
  <span class="c-state" role="cell" data-tone={st.tone}>{st.text}</span>
  <span class="c-acts" role="cell">
    <button
      class="icon-btn"
      onclick={() => open('status')}
      aria-label={m.tab_status()}
      title={m.tab_status()}><Gauge size={14} /></button
    >
    <button
      class="icon-btn"
      onclick={() => open('sftp')}
      aria-label={m.tab_files()}
      title={m.tab_files()}><FolderOpen size={14} /></button
    >
    {#if server.host}
      {@const id = server.host.id}
      <button
        class="icon-btn"
        onclick={() => ui.openHostEditor(id)}
        aria-label={m.dash_edit()}
        title={m.dash_edit()}><Pencil size={13} /></button
      >
      <button
        class="icon-btn danger"
        onclick={() => (ui.deleteKeys = [hkey(id)])}
        aria-label={m.dash_delete()}
        title={m.dash_delete()}><Trash2 size={13} /></button
      >
    {/if}
  </span>
</div>

<style>
  .row {
    position: relative;
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    column-gap: 16px;
    height: var(--row-tile);
    padding: 0 10px;
    border-radius: var(--r-control);
    color: var(--text);
  }
  .row:hover {
    background: var(--hover);
  }
  .row:has(.hit:active) {
    background: var(--sel);
  }
  .row:has(.hit:focus-visible) {
    outline: 3px solid color-mix(in oklab, var(--accent) 55%, transparent);
    outline-offset: -1px;
  }
  .hit {
    position: absolute;
    inset: 0;
    border: 0;
    border-radius: inherit;
    background: none;
    cursor: pointer;
    outline: none;
  }
  [role='cell'] {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    pointer-events: none;
  }
  .c-name {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .c-name b {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }
  .noflag {
    display: grid;
    place-items: center;
    flex: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--fill-2);
    color: var(--muted);
  }
  .c-addr,
  .c-folder,
  .c-ping {
    color: var(--muted);
  }
  .c-ping {
    text-align: right;
  }
  .c-load {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    text-align: right;
    font-size: var(--fs-sm);
  }
  .c-load.stale {
    color: var(--muted);
  }
  .c-load .warm {
    color: var(--warn);
  }
  .c-load .hot {
    color: var(--danger);
    font-weight: 600;
  }
  .c-state {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .c-state[data-tone='connected'] {
    color: var(--ok);
    font-weight: 500;
  }
  .c-state[data-tone='failed'],
  .c-state[data-tone='offline'] {
    color: var(--danger);
  }
  .c-acts {
    display: flex;
    justify-content: flex-end;
    gap: 0;
    opacity: 0;
    transition: opacity 120ms var(--ease);
  }
  .c-acts .icon-btn {
    pointer-events: auto;
    position: relative;
    width: 26px;
    height: 26px;
  }
  .c-acts .danger:hover {
    color: var(--danger);
  }
  .row:hover .c-acts,
  .row:focus-within .c-acts {
    opacity: 1;
  }
  @media (hover: none) {
    .c-acts {
      opacity: 1;
    }
  }
</style>
