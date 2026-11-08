<script lang="ts">
  import { Check, Clock, X } from '@lucide/svelte';
  import type { ConnectStep } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import type { ConnectLog } from './connectlog.svelte';
  import { diagnose, type Diagnosis } from './diagnose';

  /** The steps of a connect with their times, and, when it fails, where and why it stopped. */
  let {
    log,
    name,
    failed = false,
    onclosetab,
    onretry
  }: {
    log: ConnectLog;
    name: string;
    failed?: boolean;
    onclosetab: () => void;
    onretry: () => void;
  } = $props();

  /** Seconds after which the backend gives up on a step. */
  const LIMIT: Partial<Record<ConnectStep, number>> = {
    Resolving: 15,
    Connecting: 15,
    Handshake: 30
  };

  const stepLabel = (s: ConnectStep) =>
    ({
      Resolving: m.term_step_resolving,
      Connecting: m.term_step_connecting,
      Handshake: m.term_step_handshake,
      Authenticating: m.term_step_authenticating,
      OpeningChannel: m.term_step_channel
    })[s]();

  const time = (ms: number) =>
    ms < 1000 ? m.cx_ms({ n: Math.round(ms) }) : m.cx_sec({ n: (ms / 1000).toFixed(1) });

  const attemptText = (f: string | null) =>
    f === null
      ? m.cx_attempt_ok()
      : f === 'Timeout'
        ? m.cx_attempt_Timeout()
        : f === 'ConnectionRefused'
          ? m.cx_attempt_ConnectionRefused()
          : f === 'HostUnreachable' || f === 'NetworkUnreachable'
            ? m.cx_attempt_unreachable()
            : m.cx_attempt_other({ kind: f });

  const diag = $derived<Diagnosis | null>(
    failed && log.error
      ? diagnose({
          step: log.last?.step ?? null,
          error: log.error,
          attempts: log.last?.attempts ?? [],
          viaJump: log.viaJump
        })
      : null
  );

  function explain(d: Diagnosis): { title: string; hint: string } {
    const addr = d.addr ?? '—';
    const p = { addr, detail: d.detail };
    switch (d.kind) {
      case 'dns':
        return { title: m.cx_d_dns_t(), hint: m.cx_d_dns_h() };
      case 'dns_timeout':
        return { title: m.cx_d_dns_timeout_t(), hint: m.cx_d_dns_timeout_h() };
      case 'refused':
        return { title: m.cx_d_refused_t(), hint: m.cx_d_refused_h(p) };
      case 'unreachable':
        return { title: m.cx_d_unreachable_t(), hint: m.cx_d_unreachable_h(p) };
      case 'tcp_timeout':
        return { title: m.cx_d_tcp_timeout_t(), hint: m.cx_d_tcp_timeout_h(p) };
      case 'tcp_other':
        return { title: m.cx_d_tcp_other_t(), hint: m.cx_d_tcp_other_h(p) };
      case 'jump':
        return { title: m.cx_d_jump_t(), hint: m.cx_d_jump_h() };
      case 'hs_timeout':
        return { title: m.cx_d_hs_timeout_t(), hint: m.cx_d_hs_timeout_h() };
      case 'hs_dropped':
        return { title: m.cx_d_hs_dropped_t(), hint: m.cx_d_hs_dropped_h() };
      case 'hs_algo':
        return { title: m.cx_d_hs_algo_t(), hint: m.cx_d_hs_algo_h(p) };
      case 'hs_protocol':
        return { title: m.cx_d_hs_protocol_t(), hint: m.cx_d_hs_protocol_h() };
      case 'auth':
        return {
          title: m.cx_d_auth_t(),
          hint: d.methods.length
            ? m.cx_d_auth_h({ methods: d.methods.join(', ') })
            : m.cx_d_auth_none_h()
        };
      case 'channel':
        return { title: m.cx_d_channel_t(), hint: m.cx_d_channel_h() };
      case 'cancelled':
        return { title: m.cx_d_cancelled_t(), hint: '' };
      case 'no_user':
        return { title: m.cx_d_no_user_t(), hint: m.cx_d_no_user_h() };
      case 'no_host':
        return { title: m.cx_d_no_host_t(), hint: m.cx_d_no_host_h() };
      case 'prepare':
        return { title: m.cx_d_prepare_t(), hint: m.cx_d_prepare_h() };
      default:
        return { title: m.cx_d_generic_t(), hint: '' };
    }
  }

  const why = $derived(diag ? explain(diag) : null);
  const manyHops = $derived(log.hops.length > 1);

  async function copyReport() {
    const text = log.report(name, why ? `${why.title} ${why.hint}`.trim() : null);
    try {
      await navigator.clipboard.writeText(text);
      ui.toast(m.cx_copied());
    } catch {
      /* clipboard unavailable */
    }
  }
</script>

<div class="cx" role={failed ? 'alert' : 'status'}>
  <h3>{why ? why.title : m.term_connecting_to({ name })}</h3>
  {#if why && why.hint}<p class="hint">{why.hint}</p>{/if}

  {#if log.rows.length}
    <ol class="steps">
      {#each log.rows as r (r.id)}
        <li class={r.state}>
          <span class="mark" aria-hidden="true">
            {#if r.state === 'ok'}<Check size={13} strokeWidth={2.5} />
            {:else if r.state === 'failed'}<X size={13} strokeWidth={2.5} />
            {:else if r.state === 'waiting'}<Clock size={12} strokeWidth={2.25} />
            {:else}<span class="pulse"></span>{/if}
          </span>
          <span class="lbl">
            {stepLabel(r.step)}
            {#if manyHops}<span class="hop">{r.hop}</span>{/if}
          </span>
          <span class="meta tnum">
            {#if r.state === 'failed'}<span class="bad">{m.cx_stopped_here()}</span>
            {:else if r.state === 'waiting'}{m.cx_waiting_user()}
            {:else if r.state === 'active'}
              {#if log.elapsed(r) >= 2000}
                {@const n = Math.floor(log.elapsed(r) / 1000)}
                {@const max = LIMIT[r.step]}
                {max ? m.cx_waiting({ n, max }) : m.cx_waiting_open({ n })}
              {/if}
            {:else}{time(log.elapsed(r))}{/if}
          </span>
          {#if r.attempts.length}
            <ul class="att">
              {#each r.attempts as a, i (i)}
                <li>
                  <span class="addr">{a.addr}</span>
                  <span class:bad={a.failure !== null}>{attemptText(a.failure)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}

  {#if failed}
    <div class="row">
      <button class="btn btn-ghost" onclick={onclosetab}>{m.term_close_tab()}</button>
      <button class="btn btn-ghost" onclick={() => void copyReport()}>{m.cx_copy_report()}</button>
      <button class="btn btn-primary" onclick={onretry}>{m.term_retry()}</button>
    </div>
  {/if}
</div>

<style>
  .cx {
    width: min(420px, calc(100% - 32px));
    padding: 18px 18px 16px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
    text-align: start;
    pointer-events: auto;
    animation: rise 200ms var(--ease) 120ms backwards;
  }
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .hint {
    margin: 6px 0 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--muted);
    text-wrap: pretty;
  }
  .steps {
    display: grid;
    gap: 2px;
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
  }
  .steps > li {
    display: grid;
    grid-template-columns: 18px 1fr auto;
    align-items: center;
    column-gap: 10px;
    min-height: 26px;
    font-size: 13px;
  }
  .mark {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    color: var(--muted);
  }
  .ok .mark {
    color: var(--ok);
  }
  .failed .mark {
    color: #fff;
    background: var(--danger);
  }
  .waiting .mark {
    color: var(--warn);
  }
  .lbl {
    min-width: 0;
  }
  .active .lbl,
  .failed .lbl {
    font-weight: 600;
  }
  .hop {
    margin-inline-start: 6px;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 8%, transparent);
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
  }
  .meta {
    font-size: 12px;
    color: var(--muted);
  }
  .bad {
    color: var(--danger);
  }
  .att {
    grid-column: 2 / -1;
    display: grid;
    gap: 1px;
    margin: 0 0 4px;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--muted);
  }
  .att li {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .addr {
    font-family: var(--font-mono);
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .cx,
    .pulse {
      animation: none;
    }
  }
</style>
