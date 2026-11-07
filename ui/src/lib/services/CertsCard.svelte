<script lang="ts">
  import { ShieldCheck } from '@lucide/svelte';
  import { services } from './services.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import * as m from '$lib/paraglide/messages.js';
  import { certTone, daysLeft } from './ports';

  /** TLS certificates found on the server and how long they have left (ADR-012). */
  let { sid }: { sid: string } = $props();

  $effect(() => services.watch('certs', sid));
  const report = $derived(services.certs[sid] ?? null);

  const date = (ts: number) =>
    new Date(ts).toLocaleDateString(getLocale(), {
      day: 'numeric',
      month: 'short',
      year: 'numeric'
    });
</script>

<section class="card ssl" style:--tone="var(--c-green)">
  <h3><ShieldCheck size={15} />{m.ssl_title()}</h3>
  {#if !report}
    <p class="note">{m.ssl_loading()}</p>
  {:else if report.no_openssl}
    <p class="note">{m.ssl_no_openssl()}</p>
  {:else}
    {#if report.certs.length}
      <ul class="rows">
        {#each report.certs as c (c.path)}
          {@const days = daysLeft(c.not_after)}
          <li class="row" data-tone={certTone(days)} title={c.path}>
            <div class="who">
              <span class="name">{c.names[0] ?? c.path}</span>
              <span class="sub"
                >{#if c.names.length > 1}+{c.names.length - 1}{#if c.issuer}<span class="sep"
                      >·</span
                    >{/if}{/if}{c.issuer ?? ''}</span
              >
            </div>
            <div class="when">
              <span class="badge tnum"
                >{days < 0 ? m.ssl_expired() : m.ssl_days({ count: days })}</span
              >
              <span class="sub tnum">{m.ssl_until({ date: date(c.not_after) })}</span>
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="note">{m.ssl_none()}</p>
    {/if}
    {#if report.denied}<p class="note">{m.ssl_denied()}</p>{/if}
  {/if}
</section>

<style>
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .rows {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 8px;
    border-radius: var(--r-inner);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .who {
    flex: 1;
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
  .sub {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    display: grid;
    justify-items: end;
    gap: 2px;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--ok) 14%, transparent);
    color: var(--ok);
    font-size: 11.5px;
    font-weight: 600;
  }
  [data-tone='warn'] .badge {
    background: color-mix(in oklab, var(--warn) 16%, transparent);
    color: color-mix(in oklab, var(--warn) 70%, var(--text));
  }
  [data-tone='bad'] .badge {
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .sep {
    margin: 0 0.35em;
  }
</style>
