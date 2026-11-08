<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { dockerRes } from './dockerres.svelte';
  import ResGate from './ResGate.svelte';

  /** Compose projects of the server with their containers (ADR-016). */
  let { sid }: { sid: string } = $props();

  $effect(() => dockerRes.watch('compose', sid));
  const report = $derived(dockerRes.compose[sid] ?? null);
  const tone = (state: string) =>
    state === 'running' ? 'ok' : state === 'restarting' || state === 'paused' ? 'warn' : 'off';
</script>

<ResGate
  access={report?.access ?? null}
  count={report?.projects.length ?? 0}
  none={m.dk_compose_none()}
>
  <ul class="res">
    {#each report?.projects ?? [] as p (p.name)}
      <li class="proj">
        <div class="who">
          <span class="name">{p.name}</span>
          <span class="sub" title={p.dir ?? ''}>{p.dir ?? ''}</span>
        </div>
        <span
          class="pill"
          class:off={p.running === 0}
          style:--tone={p.running === p.total ? 'var(--ok)' : 'var(--warn)'}
          >{m.dk_compose_count({ running: p.running, total: p.total })}</span
        >
        <ul class="svcs">
          {#each p.services as s (s.container)}
            <li data-tone={tone(s.state)} title="{s.container} · {s.status}">
              <i aria-hidden="true"></i>{s.service || s.container}
            </li>
          {/each}
        </ul>
      </li>
    {/each}
  </ul>
</ResGate>

<style>
  .res > li.proj {
    grid-template-columns: minmax(0, 1fr) auto;
    row-gap: 6px;
    padding-block: 8px;
  }
  .svcs {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .svcs li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px 2px 7px;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 11.5px;
  }
  .svcs i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
    opacity: 0.5;
  }
  [data-tone='ok'] i {
    background: var(--ok);
    opacity: 1;
  }
  [data-tone='warn'] i {
    background: var(--warn);
    opacity: 1;
  }
  [data-tone='off'] {
    color: var(--muted);
  }
</style>
