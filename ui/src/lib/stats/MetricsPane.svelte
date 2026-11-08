<script lang="ts">
  import { Clock, History, ServerCog } from '@lucide/svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { ago, uptime } from './format';
  import SpecLine from './SpecLine.svelte';
  import StatsCards from './StatsCards.svelte';

  /** Live metrics of a server: compact in the side panel, a card grid on the Status page. */
  let { group, wide = false }: { group: TabGroup; wide?: boolean } = $props();

  const sid = $derived(sessions.groupSession(group));
  $effect(() => {
    if (sid) return stats.watch(sid);
  });
  const live = $derived(stats.get(sid));
  const hostId = $derived(group.current.hostId);
  // Remember live samples; show the remembered one until the session reports.
  $effect(() => {
    if (hostId && live?.latest && !live.stale) lastSeen.record(hostId, live);
  });
  const snap = $derived(lastSeen.get(hostId));
  const s = $derived(live?.latest ? live : snap ? lastSeen.toSeries(snap) : null);
  const remembered = $derived(!live?.latest && !!snap);
  const d = $derived(s?.latest ?? null);
  const phase = $derived(sessions.groupPhase(group));
  const time = (ts: number) =>
    new Date(ts).toLocaleString(getLocale(), {
      day: 'numeric',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit'
    });
</script>

{#if remembered && d && s && snap}
  <p class="last" title={m.stat_last_hint({ time: time(snap.at) })}>
    <History size={13} />{m.stat_last({ when: ago(snap.at, getLocale()) })}
    {#if phase === 'connecting'}<span class="dot" data-phase="connecting"></span>{/if}
  </p>
  <div class="rspec"><SpecLine {d} model={wide} /></div>
  <StatsCards {s} {d} {wide} />
{:else if !sid}
  <div class="state">
    <span class="state-icon"><ServerCog size={22} /></span>
    <p>
      {phase === 'connecting'
        ? m.stat_connecting()
        : phase === 'failed'
          ? m.dash_connection_error()
          : m.stat_no_session()}
    </p>
  </div>
{:else if s?.unsupported}
  <div class="state"><p>{m.stat_unsupported()}</p></div>
{:else if !d || !s}
  <div class="state">
    <span class="dot" data-phase="connecting"></span>
    <p>{m.stat_waiting()}</p>
  </div>
{:else}
  <header class="head" class:wide>
    <div class="facts tnum">
      {#if d.os_name}<span class="os">{d.os_name}</span>{/if}
      <SpecLine {d} model={wide} />
      <span title={m.stat_uptime()}
        ><Clock size={13} />{uptime(d.uptime_s, m.unit_d(), m.unit_h(), m.unit_m())}</span
      >
    </div>
    <span
      class="live"
      class:stale={s.stale}
      title={s.updatedAt
        ? m.stat_sampled({ time: new Date(s.updatedAt).toLocaleTimeString() })
        : ''}><i></i>{s.stale ? m.stat_stale() : wide ? m.stat_live() : m.stat_live_short()}</span
    >
  </header>
  <StatsCards {s} {d} {wide} />
{/if}

<style>
  .last {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 12px;
    padding: 8px 12px;
    border-radius: var(--r-control);
    background: color-mix(in oklab, var(--warn) 12%, transparent);
    color: color-mix(in oklab, var(--warn) 60%, var(--text));
    font-size: 12px;
    font-weight: 500;
  }
  .rspec {
    margin: 0 0 12px;
  }
  .last .dot {
    margin-left: auto;
  }
  .state {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 48px 16px;
    color: var(--muted);
    text-align: center;
  }
  .state p {
    margin: 0;
    max-width: 280px;
    font-size: 13px;
    line-height: 1.45;
  }
  .state-icon {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 16px;
    background: var(--surface);
    box-shadow: var(--card-shadow);
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 12px;
  }
  .head.wide {
    margin-bottom: 18px;
  }
  .facts {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    color: var(--muted);
    font-size: 12px;
  }
  .facts span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .os {
    padding: 4px 10px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--c-purple) 13%, transparent);
    color: var(--c-purple);
    font-weight: 600;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    padding-top: 4px;
    color: var(--ok);
    font-size: 11px;
    font-weight: 600;
  }
  .live i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: breathe 2.4s ease-in-out infinite;
  }
  .live.stale {
    color: var(--warn);
  }
  .live.stale i {
    animation: none;
  }
  @keyframes breathe {
    50% {
      opacity: 0.35;
    }
  }
</style>
