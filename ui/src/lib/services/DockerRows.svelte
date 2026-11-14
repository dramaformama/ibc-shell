<script lang="ts">
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { Dialog } from 'bits-ui';
  import { Play, RotateCw, ScrollText, Square } from '@lucide/svelte';
  import { api, type Container, type DockerAction, type DockerReport } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { bytes, decimal } from '$lib/stats/format';
  import DockerLogs from './DockerLogs.svelte';
  import { portChips } from './ports';
  import { services } from './services.svelte';

  /** Container rows of one server with logs and start / stop / restart (ADR-012). */
  let { sid, report }: { sid: string; report: DockerReport | null } = $props();

  let busy = $state<Record<string, boolean>>({});
  let ask = $state<{ c: Container; action: DockerAction } | null>(null);
  let logsOf = $state<Container | null>(null);

  async function act(c: Container, action: DockerAction) {
    ask = null;
    busy = { ...busy, [c.id]: true };
    const r = await api.dockerAction(sid, c.id, action).catch(() => null);
    busy = { ...busy, [c.id]: false };
    if (r?.status === 'ok') ui.toast(m.dk_done({ name: c.name }));
    else
      ui.toast(
        r?.status === 'error' && r.error.code === 'Validation'
          ? r.error.detail
          : m.dash_connection_error(),
        'error'
      );
    await services.refreshDocker(sid);
  }

  const tone = (state: string) =>
    state === 'running' ? 'ok' : state === 'restarting' || state === 'paused' ? 'warn' : 'off';
  const verb = (a: DockerAction) => (a === 'Stop' ? m.dk_stop() : m.dk_restart());
</script>

{#if !report}
  <Skeleton rows={4} label={m.dk_loading()} />
{:else if report.access === 'Missing'}
  <p class="note">{m.dk_missing()}</p>
{:else if report.access === 'Denied'}
  <p class="note">{m.dk_denied()}</p>
{:else if report.access === 'Down'}
  <p class="note">{m.dk_down()}</p>
{:else if !report.containers.length}
  <p class="note">{m.dk_none()}</p>
{:else}
  <ul class="rows">
    {#each report.containers as c (c.id)}
      {@const chips = portChips(c.ports)}
      <li class="row" data-tone={tone(c.state)}>
        <i class="dot" aria-hidden="true"></i>
        <div class="who">
          <span class="name">{c.name}</span>
          <span class="sub" title={c.image}>{c.image}</span>
        </div>
        <span class="st">{c.status}</span>
        <span class="ports">
          {#each chips.slice(0, 3) as p (p.text + p.proto)}
            <span class="chip" class:local={p.local || p.internal} title="{p.text}/{p.proto}"
              >{p.text}{#if p.proto !== 'tcp'}<small>/{p.proto}</small>{/if}</span
            >
          {/each}
          {#if chips.length > 3}<span class="chip local">+{chips.length - 3}</span>{/if}
        </span>
        <span class="use tnum">
          {#if c.cpu_pct != null}<span>{decimal(c.cpu_pct, 1)}%</span>{/if}
          {#if c.mem_bytes != null}<span class="muted">{bytes(c.mem_bytes)}</span>{/if}
        </span>
        <span class="acts">
          <button
            class="icon-btn"
            title={m.dk_logs()}
            aria-label={m.dk_logs()}
            onclick={() => (logsOf = c)}><ScrollText size={15} /></button
          >
          {#if c.state === 'running'}
            <button
              class="icon-btn"
              title={m.dk_restart()}
              aria-label={m.dk_restart()}
              disabled={busy[c.id]}
              onclick={() => (ask = { c, action: 'Restart' })}
              ><RotateCw size={15} class={busy[c.id] ? 'spin' : ''} /></button
            >
            <button
              class="icon-btn"
              title={m.dk_stop()}
              aria-label={m.dk_stop()}
              disabled={busy[c.id]}
              onclick={() => (ask = { c, action: 'Stop' })}><Square size={14} /></button
            >
          {:else}
            <button
              class="icon-btn"
              title={m.dk_start()}
              aria-label={m.dk_start()}
              disabled={busy[c.id]}
              onclick={() => act(c, 'Start')}
              ><Play size={15} class={busy[c.id] ? 'spin' : ''} /></button
            >
          {/if}
        </span>
      </li>
    {/each}
  </ul>
{/if}

{#if logsOf}
  <DockerLogs {sid} container={logsOf} onclose={() => (logsOf = null)} />
{/if}

<Dialog.Root open={ask !== null} onOpenChange={(o) => !o && (ask = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="confirm">
      {#if ask}
        <Dialog.Title
          >{ask.action === 'Stop'
            ? m.dk_confirm_stop({ name: ask.c.name })
            : m.dk_confirm_restart({ name: ask.c.name })}</Dialog.Title
        >
        <Dialog.Description>{m.dk_confirm_body()}</Dialog.Description>
        <div class="dk-actions">
          <Dialog.Close class="btn btn-ghost">{m.action_cancel()}</Dialog.Close>
          <button class="btn btn-primary" onclick={() => ask && act(ask.c, ask.action)}
            >{verb(ask.action)}</button
          >
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .rows {
    container-type: inline-size;
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
  }
  .row {
    display: grid;
    grid-template-columns: 8px minmax(120px, 1.4fr) minmax(0, 1fr) minmax(0, 1fr) 92px auto;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 8px;
    border-radius: var(--r-inner);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
    opacity: 0.5;
  }
  [data-tone='ok'] .dot {
    background: var(--ok);
    opacity: 1;
  }
  [data-tone='warn'] .dot {
    background: var(--warn);
    opacity: 1;
  }
  .who {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub,
  .st {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  [data-tone='off'] .name {
    color: var(--muted);
  }
  .ports {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .chip {
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--fill-2);
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .chip.local {
    background: var(--surface-2);
    color: var(--muted);
  }
  .chip small {
    font-weight: 500;
    opacity: 0.7;
  }
  .use {
    display: grid;
    justify-items: end;
    line-height: 1.25;
  }
  .muted {
    color: var(--muted);
    font-size: 11.5px;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    /* Fixed: a stopped container has fewer buttons, the columns must not move. */
    width: calc(var(--control-h) * 3 + 4px);
    opacity: 0.55;
    transition: opacity 120ms var(--ease);
  }
  .row:hover .acts,
  .acts:focus-within {
    opacity: 1;
  }
  .acts :global(.spin) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      rotate: 360deg;
    }
  }
  @container (max-width: 760px) {
    .row {
      grid-template-columns: 8px minmax(0, 1fr) auto auto;
    }
    .st,
    .ports {
      display: none;
    }
  }
  .dk-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
