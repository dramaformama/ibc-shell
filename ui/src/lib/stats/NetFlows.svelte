<script lang="ts">
  import { ArrowDownLeft, ArrowUpRight, Cable } from '@lucide/svelte';
  import type { HostStats } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { flowKey, type Series } from '$lib/stores/stats.svelte';
  import { bytes, rate } from './format';

  /**
   * Who talks to the server and on which port (ADR-009 amendment 2): inbound / outbound TCP
   * grouped by port and remote address, plus the listening sockets.
   */
  let { s, d, wide = false }: { s: Series; d: HostStats; wide?: boolean } = $props();

  type View = 'in' | 'out' | 'ports';
  let view = $state<View>('in');

  const WELL_KNOWN: Record<number, string> = {
    22: 'ssh',
    25: 'smtp',
    53: 'dns',
    80: 'http',
    123: 'ntp',
    443: 'https',
    993: 'imaps',
    3306: 'mysql',
    5432: 'postgres',
    6379: 'redis',
    8080: 'http-alt',
    8443: 'https-alt'
  };
  const program = $derived(
    new Map(d.listen.filter((l) => l.process).map((l) => [l.port, l.process!]))
  );
  const service = (port: number, inbound: boolean) =>
    (inbound ? program.get(port) : undefined) ?? WELL_KNOWN[port] ?? '';

  const hasRates = $derived(Object.keys(s.flowRates).length > 0);
  const rows = $derived(
    d.flows
      .filter((f) => f.inbound === (view === 'in'))
      .map((f) => {
        const r = s.flowRates[flowKey(f)];
        return { ...f, rin: r?.[0] ?? 0, rout: r?.[1] ?? 0, total: f.bytes_in + f.bytes_out };
      })
      .sort((a, b) =>
        hasRates ? b.rin + b.rout - (a.rin + a.rout) || b.total - a.total : b.total - a.total
      )
  );
  const limit = $derived(wide ? 40 : 8);
  const top = $derived(Math.max(1, ...rows.map((r) => (hasRates ? r.rin + r.rout : r.total))));
  const counts = $derived({
    in: d.flows.filter((f) => f.inbound).reduce((n, f) => n + f.conns, 0),
    out: d.flows.filter((f) => !f.inbound).reduce((n, f) => n + f.conns, 0)
  });
  const local = (addr: string) => addr.startsWith('127.') || addr === '::1';
  const anyAddr = (addr: string) => addr === '0.0.0.0' || addr === '::' || addr === '*';
  const noData = $derived(!d.listen.length && !d.flows.length);
</script>

<section class="card flows-card" class:wide style:--tone="var(--accent)">
  <h3>
    <Cable size={15} />{m.net_title()}
    {#if d.conns}<span class="count tnum">{m.net_conns({ count: d.conns })}</span>{/if}
  </h3>
  {#if noData}
    <p class="note">{m.net_no_ss()}</p>
  {:else}
    <div class="seg" role="tablist" aria-label={m.net_title()}>
      <button role="tab" aria-selected={view === 'in'} onclick={() => (view = 'in')}
        ><span class="t">{m.net_in()}</span><span class="n tnum">{counts.in}</span></button
      >
      <button role="tab" aria-selected={view === 'out'} onclick={() => (view = 'out')}
        ><span class="t">{m.net_out()}</span><span class="n tnum">{counts.out}</span></button
      >
      <button role="tab" aria-selected={view === 'ports'} onclick={() => (view = 'ports')}
        ><span class="t">{m.net_ports()}</span><span class="n tnum">{d.listen.length}</span></button
      >
    </div>

    {#if view === 'ports'}
      <ul class="list tnum">
        {#each d.listen as l (`${l.proto}${l.port}${l.addr}${l.process}`)}
          <li class="port-row">
            <span class="port">{l.port}<small>{l.proto}</small></span>
            <span class="what">{l.process ?? WELL_KNOWN[l.port] ?? '—'}</span>
            <span class="where" class:warn={!local(l.addr)}
              >{local(l.addr) ? m.net_local() : anyAddr(l.addr) ? m.net_all() : l.addr}</span
            >
          </li>
        {/each}
      </ul>
      <p class="note">{m.net_udp_note()}</p>
    {:else if !rows.length}
      <p class="note">{m.net_none()}</p>
    {:else}
      <ul class="list tnum">
        {#each rows.slice(0, limit) as f (flowKey(f))}
          {@const v = hasRates ? f.rin + f.rout : f.total}
          <li class="flow" title="{f.peer}:{f.port} · {bytes(f.bytes_in)} ↓ {bytes(f.bytes_out)} ↑">
            <span class="port"
              >{f.port}{#if service(f.port, f.inbound)}<small>{service(f.port, f.inbound)}</small
                >{/if}</span
            >
            <span class="peer mono"
              >{#if f.inbound}<ArrowDownLeft size={12} />{:else}<ArrowUpRight
                  size={12}
                />{/if}{f.peer}{#if f.conns > 1}<em>×{f.conns}</em>{/if}</span
            >
            <span class="rate">
              {#if hasRates}
                <span>↓ {rate(f.rin).join(' ')}</span><span>↑ {rate(f.rout).join(' ')}</span>
              {:else}
                <span>↓ {bytes(f.bytes_in)}</span><span>↑ {bytes(f.bytes_out)}</span>
              {/if}
            </span>
            <i class="bar" style:--w="{Math.max(2, (v / top) * 100)}%"></i>
          </li>
        {/each}
      </ul>
      {#if rows.length > limit}<p class="note">{m.net_more({ count: rows.length - limit })}</p>{/if}
      {#if !hasRates}<p class="note">{m.net_since()}</p>{/if}
    {/if}
  {/if}
</section>

<style>
  .count {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
  }
  /* Three tabs with counters share the card by content, so a long label never leaves its pill. */
  .seg {
    width: 100%;
    container-type: inline-size;
  }
  .seg button {
    flex: 1 1 auto;
    min-width: 0;
    gap: 5px;
    padding-inline: 8px;
    justify-content: center;
    white-space: nowrap;
  }
  .t {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .n {
    flex: none;
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }
  /* Too narrow for three counters: only the open tab keeps its number. */
  @container (max-width: 310px) {
    .seg button[aria-selected='false'] .n {
      display: none;
    }
  }
  .list {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }
  .flow,
  .port-row {
    position: relative;
    display: grid;
    grid-template-columns: 60px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 4px 8px;
    border-radius: var(--r-inner);
  }
  .flow:hover,
  .port-row:hover {
    background: var(--surface-2);
  }
  .port {
    display: grid;
    line-height: 1.2;
    font-weight: 600;
  }
  .port small {
    overflow: hidden;
    color: var(--muted);
    font-size: 10.5px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .peer {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .peer :global(svg) {
    flex: none;
    color: var(--tone);
  }
  .peer em {
    color: var(--muted);
    font-style: normal;
    font-size: 11px;
  }
  .rate {
    display: grid;
    justify-items: end;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.3;
    white-space: nowrap;
  }
  .wide .flow {
    grid-template-columns: 110px minmax(0, 1fr) auto;
    gap: 10px;
  }
  /* The address is the part that must stay whole: shrink the rest first. */
  .flows-card:not(.wide) .peer {
    font-size: 11.5px;
    gap: 3px;
  }
  .flows-card:not(.wide) .rate {
    font-size: 10.5px;
  }
  .wide .rate {
    grid-auto-flow: column;
    grid-auto-columns: 92px;
    gap: 12px;
    font-size: 12px;
  }
  .wide .peer {
    font-size: 12.5px;
  }
  .bar {
    position: absolute;
    inset: auto 8px 1px 8px;
    height: 2px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      color-mix(in oklab, var(--tone) 70%, transparent) var(--w),
      transparent var(--w)
    );
  }
  .what {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .where {
    color: var(--muted);
    font-size: 11px;
  }
  .where.warn {
    color: var(--text);
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
</style>
