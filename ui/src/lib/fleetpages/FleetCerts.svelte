<script lang="ts">
  import { ShieldCheck } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import * as m from '$lib/paraglide/messages.js';
  import { certTone, daysLeft } from '$lib/services/ports';
  import { services } from '$lib/services/services.svelte';
  import { connectedServers } from './connected.svelte';
  import './page.css';

  /** Every certificate of every connected server, soonest expiry first (ADR-012). */
  const live = $derived(connectedServers());
  $effect(() => {
    const stops = live.map((s) => services.watch('certs', s.sid));
    return () => stops.forEach((stop) => stop());
  });
  const rows = $derived(
    live
      .flatMap((s) => (services.certs[s.sid]?.certs ?? []).map((c) => ({ ...c, s })))
      .sort((a, b) => a.not_after - b.not_after)
  );
  const soon = $derived(rows.filter((r) => daysLeft(r.not_after) <= 21).length);
  const denied = $derived(live.filter((s) => (services.certs[s.sid]?.denied ?? 0) > 0));
  const loading = $derived(live.some((s) => !services.certs[s.sid]));
  const date = (ts: number) =>
    new Date(ts).toLocaleDateString(getLocale(), {
      day: 'numeric',
      month: 'short',
      year: 'numeric'
    });
</script>

<section class="fpage" aria-label={m.fleet_domains()}>
  <header>
    <h1>{m.fleet_domains()}</h1>
    <p class="subtitle tnum">{m.fleet_ssl_counts({ total: rows.length, soon })}</p>
  </header>
  {#if !live.length}
    <div class="blank">
      <ShieldCheck size={28} />
      <p>{m.fleet_need_connect()}</p>
    </div>
  {:else if !rows.length}
    <div class="blank">
      <ShieldCheck size={28} />
      <p>{loading ? m.ssl_loading() : m.ssl_none()}</p>
    </div>
  {:else}
    <section class="card list">
      {#each rows as r (r.s.key + r.path)}
        {@const days = daysLeft(r.not_after)}
        <div class="row" data-tone={certTone(days)} title={r.path}>
          <div class="who">
            <span class="name">{r.names[0] ?? r.path}</span>
            <span class="sub"
              >{#if r.names.length > 1}+{r.names.length - 1}<span class="sep">·</span
                >{/if}{r.issuer ?? ''}</span
            >
          </div>
          <span class="srv"><Flag code={r.s.group.flag} size={15} />{r.s.group.label}</span>
          <span class="until tnum">{m.ssl_until({ date: date(r.not_after) })}</span>
          <span class="badge tnum">{days < 0 ? m.ssl_expired() : m.ssl_days({ count: days })}</span>
        </div>
      {/each}
    </section>
  {/if}
  {#if denied.length}
    <p class="subtitle">
      {m.fleet_ssl_denied({ names: denied.map((s) => s.group.label).join(', ') })}
    </p>
  {/if}
</section>

<style>
  .list {
    display: grid;
    gap: 2px;
    padding: 8px;
    margin-bottom: 14px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr) auto 76px;
    align-items: center;
    gap: 16px;
    min-height: 50px;
    padding: 6px 12px;
    border-radius: var(--r-inner);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .who {
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub,
  .until {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sep {
    margin: 0 0.35em;
  }
  .srv {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12.5px;
  }
  .badge {
    justify-self: end;
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
  @media (max-width: 900px) {
    .row {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .srv,
    .until {
      display: none;
    }
  }
</style>
