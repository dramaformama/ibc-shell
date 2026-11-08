<script lang="ts">
  import EmptyState from '$lib/components/EmptyState.svelte';
  import { Clock, Cpu, History, ServerCog } from '@lucide/svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type TabGroup } from '$lib/stores/sessions.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { ago, uptime } from './format';
  import SpecLine from './SpecLine.svelte';
  import { cpuShort } from './spec';
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

{#snippet facts(d: NonNullable<typeof s>['latest'] & object)}
  <div class="meta">
    <div class="facts tnum">
      {#if d.os_name}<span class="os">{d.os_name}</span>{/if}
      <SpecLine {d} inline />
      <span title={m.stat_uptime()}
        ><Clock size={13} />{uptime(d.uptime_s, m.unit_d(), m.unit_h(), m.unit_m())}</span
      >
    </div>
    {#if d.cpu_model}
      <p class="model tnum" title={d.cpu_model}>
        <Cpu size={13} /><span>{cpuShort(d.cpu_model)}</span>
      </p>
    {/if}
  </div>
{/snippet}

{#key group.key}
  <div class="enter">
    {#if remembered && d && s && snap}
      <p class="last" title={m.stat_last_hint({ time: time(snap.at) })}>
        <History size={13} />{m.stat_last({ when: ago(snap.at, getLocale()) })}
        {#if phase === 'connecting'}<span class="dot" data-phase="connecting"></span>{/if}
      </p>
      <header class="head" class:wide>{@render facts(d)}</header>
      <StatsCards {s} {d} {wide} />
    {:else if !sid}
      <EmptyState
        icon={ServerCog}
        busy={phase === 'connecting'}
        title={phase === 'connecting'
          ? m.stat_connecting()
          : phase === 'failed'
            ? m.dash_connection_error()
            : m.stat_no_session()}
      />
    {:else if s?.unsupported}
      <EmptyState icon={ServerCog} title={m.stat_unsupported()} />
    {:else if !d || !s}
      <EmptyState busy title={m.stat_waiting()} />
    {:else}
      <header class="head" class:wide>
        {@render facts(d)}
        <span
          class="live"
          class:stale={s.stale}
          title={s.updatedAt
            ? m.stat_sampled({ time: new Date(s.updatedAt).toLocaleTimeString() })
            : ''}
          ><i></i>{s.stale ? m.stat_stale() : wide ? m.stat_live() : m.stat_live_short()}</span
        >
      </header>
      <StatsCards {s} {d} {wide} />
    {/if}
  </div>
{/key}

<style>
  /* Switching servers: the cards settle in, one short move, no stagger. */
  .enter {
    animation: enter 150ms var(--ease) backwards;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
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
  .last .dot {
    margin-left: auto;
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
  .meta {
    flex: 1;
    min-width: 0;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    min-height: 26px;
    color: var(--muted);
    font-size: 12px;
  }
  .facts :global(span:not(.os)) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }
  .model {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
  }
  .model span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .os {
    display: inline-flex;
    align-items: center;
    height: 26px;
    padding: 0 11px;
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
    height: 26px;
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
