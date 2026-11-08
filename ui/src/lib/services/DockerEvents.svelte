<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { dockerRes } from './dockerres.svelte';
  import { eventLabel, eventTone, kindLabel } from './docker-events';
  import ResGate from './ResGate.svelte';

  /** What the daemon did in the last hour, newest first; refreshed every 5 s while open (ADR-016). */
  let { sid }: { sid: string } = $props();

  $effect(() => dockerRes.watch('events', sid));
  const report = $derived(dockerRes.events[sid] ?? null);
  const time = (t: number) =>
    new Date(t).toLocaleTimeString(getLocale(), {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false
    });
</script>

<ResGate
  access={report?.access ?? null}
  count={report?.events.length ?? 0}
  none={m.dk_events_none()}
>
  <ul class="res ev">
    {#each report?.events ?? [] as e, i (e.time + e.kind + e.action + e.name + i)}
      <li data-tone={eventTone(e.action)}>
        <div class="who">
          <span class="name" title={e.name}>{e.name || '—'}</span>
          <span class="sub">{kindLabel(e.kind)}</span>
        </div>
        <span class="act">{eventLabel(e.action)}</span>
        <span class="num sub">{time(e.time)}</span>
        <i class="mark" aria-hidden="true"></i>
      </li>
    {/each}
  </ul>
</ResGate>

<style>
  :global(.dk .res.ev > li) {
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr) 72px 8px;
  }
  @container (max-width: 640px) {
    :global(.dk .res.ev > li) {
      grid-template-columns: minmax(0, 1fr) auto auto 8px;
    }
  }
  /* Up to 200 rows: scroll inside the card instead of stretching the page. */
  .ev {
    max-height: 440px;
    overflow-y: auto;
  }
  .act {
    font-weight: 600;
  }
  [data-tone='ok'] .act {
    color: var(--ok);
  }
  [data-tone='danger'] .act {
    color: var(--danger);
  }
  [data-tone='warn'] .act {
    color: var(--warn);
  }
  [data-tone='off'] .act {
    color: var(--muted);
    font-weight: 500;
  }
  .mark {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
    opacity: 0.4;
  }
  [data-tone='ok'] .mark {
    background: var(--ok);
    opacity: 1;
  }
  [data-tone='danger'] .mark {
    background: var(--danger);
    opacity: 1;
  }
  [data-tone='warn'] .mark {
    background: var(--warn);
    opacity: 1;
  }
</style>
