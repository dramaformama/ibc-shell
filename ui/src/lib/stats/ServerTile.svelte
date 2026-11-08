<script lang="ts">
  import { FolderOpen, Gauge, Pencil, Server as ServerIcon, SquareTerminal } from '@lucide/svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { ago, ms, pct } from './format';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Server } from '$lib/fleet/model';
  import { reachability } from '$lib/fleet/reachability.svelte';
  import { connectHost } from '$lib/stores/actions';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { specText } from './spec-text';
  import { serverStatus } from './status';

  /** One saved server in the overview list: identity, availability and quick actions. */
  let { server }: { server: Server } = $props();
  $effect(() => {
    if (settings.probe && server.host && server.phase !== 'connected')
      return reachability.watch(server.host.id);
  });
  const st = $derived(
    serverStatus(server, settings.probe && reachability.data[server.id] === undefined)
  );

  const snap = $derived(lastSeen.get(server.host?.id));
  const snapMem = $derived(
    snap && snap.d.mem_total ? (snap.d.mem_used / snap.d.mem_total) * 100 : null
  );
  const snapDisk = $derived(
    snap && snap.d.disk_total ? (snap.d.disk_used / snap.d.disk_total) * 100 : null
  );

  function open(kind: 'terminal' | 'sftp' | 'status') {
    if (server.host) connectHost(server.host.id, kind);
    else if (server.group) sessions.select(server.group.current.id);
  }
</script>

<article class="tile" aria-label={server.label}>
  <button class="main" onclick={() => open('terminal')} title={m.dash_open_terminal()}>
    <span class="icon"
      >{#if server.flag}<Flag code={server.flag} size={22} />{:else}<ServerIcon size={18} />{/if}<i
        class="dot"
        data-phase={st.tone}
      ></i></span
    >
    <span class="id">
      <b>{server.label}</b>
      <small>{server.address}</small>
      {#if snap && snap.d.cores}<small class="spec tnum" title={snap.d.cpu_model ?? undefined}
          >{specText(snap.d)}</small
        >{/if}
    </span>
    {#if snap}
      <span class="snap tnum" title={m.stat_last({ when: ago(snap.at, getLocale()) })}>
        <span><small>CPU</small>{pct(snap.d.cpu_pct)}%</span>
        <span><small>RAM</small>{pct(snapMem)}%</span>
        {#if snapDisk != null}<span><small>{m.stat_disk()}</small>{pct(snapDisk)}%</span>{/if}
      </span>
    {/if}
    <span class="meta">
      <span class="st" data-tone={st.tone}>{st.text}</span>
      <small class="tnum"
        >{server.latency != null ? ms(server.latency) : server.folder || ' '}</small
      >
    </span>
  </button>
  <span class="acts">
    <button
      class="mini"
      onclick={() => open('status')}
      aria-label={m.tab_status()}
      title={m.tab_status()}><Gauge size={14} /></button
    >
    <button
      class="mini"
      onclick={() => open('sftp')}
      aria-label={m.tab_files()}
      title={m.tab_files()}><FolderOpen size={14} /></button
    >
    <button
      class="mini"
      onclick={() => open('terminal')}
      aria-label={m.tab_terminal()}
      title={m.tab_terminal()}><SquareTerminal size={14} /></button
    >
    {#if server.host}
      <button
        class="mini"
        onclick={() => ui.openHostEditor(server.host!.id)}
        aria-label={m.dash_edit()}
        title={m.dash_edit()}><Pencil size={13} /></button
      >
    {/if}
  </span>
</article>

<style>
  .tile {
    container-type: inline-size;
    position: relative;
    height: 68px;
    border-radius: var(--r-field);
    background: var(--surface);
    box-shadow: var(--card-shadow);
    transition:
      box-shadow 160ms var(--ease),
      transform 160ms var(--ease);
  }
  .tile:hover {
    box-shadow:
      var(--card-shadow),
      0 6px 18px -8px rgb(0 0 0 / 0.18);
  }
  .main {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 100%;
    padding: 0 14px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .snap {
    display: flex;
    gap: 10px;
    flex: none;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
    opacity: 0.75;
  }
  .snap span {
    display: grid;
    line-height: 1.25;
  }
  .snap small {
    color: var(--muted);
    font-size: 10px;
    font-weight: 500;
  }
  @container (max-width: 330px) {
    .snap {
      display: none;
    }
  }
  .main:active {
    transform: scale(0.99);
  }
  .icon {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .icon .dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 10px;
    height: 10px;
    border: 2px solid var(--surface);
    box-sizing: content-box;
    box-shadow: none;
  }
  .dot[data-phase='offline'] {
    background: var(--danger);
  }
  .dot[data-phase='online'] {
    background: var(--ok);
  }
  .dot[data-phase='unknown'] {
    background: color-mix(in oklab, var(--muted) 60%, transparent);
  }
  .id {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  .id b,
  .id small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .id b {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .id small,
  .meta small {
    color: var(--muted);
    font-size: 11px;
  }
  .id .spec {
    font-size: 10.5px;
    opacity: 0.85;
  }
  .meta {
    display: grid;
    justify-items: end;
    flex: none;
    max-width: 40%;
  }
  .meta small {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .st {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .st[data-tone='connected'],
  .st[data-tone='online'] {
    color: var(--ok);
  }
  .st[data-tone='offline'],
  .st[data-tone='failed'] {
    color: var(--danger);
  }
  .st[data-tone='connecting'] {
    color: var(--warn);
  }
  .acts {
    position: absolute;
    top: 50%;
    right: 10px;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 12px;
    background: var(--surface);
    box-shadow:
      var(--card-shadow),
      0 4px 12px -4px rgb(0 0 0 / 0.15);
    opacity: 0;
    transform: translateY(-50%) scale(0.96);
    pointer-events: none;
    transition:
      opacity 140ms var(--ease),
      transform 140ms var(--ease);
  }
  .tile:hover .acts,
  .tile:focus-within .acts {
    opacity: 1;
    transform: translateY(-50%);
    pointer-events: auto;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 9px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .mini:hover {
    background: var(--hover);
    color: var(--accent);
  }
</style>
