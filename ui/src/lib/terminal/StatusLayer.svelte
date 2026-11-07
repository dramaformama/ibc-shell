<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import type { Connection } from './connection.svelte';

  let { conn, name, onclosetab }: { conn: Connection; name: string; onclosetab: () => void } =
    $props();

  const stepText = $derived(
    {
      Resolving: m.term_step_resolving(),
      Connecting: m.term_step_connecting(),
      Handshake: m.term_step_handshake(),
      Authenticating: m.term_step_authenticating(),
      OpeningChannel: m.term_step_channel()
    }[conn.step ?? 'Resolving']
  );

  const failText = $derived(
    conn.reason === 'AuthFailed'
      ? m.term_failed_auth()
      : conn.reason === 'Network' || conn.reason === 'Timeout'
        ? m.term_failed_network()
        : conn.reason === 'Cancelled'
          ? m.term_failed_cancelled()
          : m.term_failed()
  );

  const blocked = $derived(conn.prompt?.kind === 'changed');
</script>

{#if conn.phase === 'connecting' && !conn.prompt}
  <div class="center" role="status">
    <p class="title">{m.term_connecting_to({ name })}</p>
    <p class="sub"><span class="pulse" aria-hidden="true"></span>{stepText}</p>
  </div>
{:else if conn.phase === 'failed' && !blocked}
  <div class="center" role="alert">
    <p class="title">{failText}</p>
    <div class="row">
      <button class="btn btn-ghost" onclick={onclosetab}>{m.term_close_tab()}</button>
      <button class="btn" onclick={() => void conn.retry()}>{m.term_retry()}</button>
    </div>
  </div>
{:else if conn.phase === 'disconnected'}
  <div class="banner" role="status">
    <span class="msg">
      <span class="dot" aria-hidden="true"></span>
      {conn.reason ? `${m.term_disconnected()} · ${conn.reason}` : m.term_disconnected()}
    </span>
    {#if conn.canReconnect}
      <button class="btn btn-primary" onclick={() => void conn.reconnect()}
        >{m.term_reconnect()}</button
      >
      <span class="hint">{m.term_reconnect_hint()}</span>
    {:else}
      <button class="btn" onclick={onclosetab}>{m.term_close_tab()}</button>
    {/if}
  </div>
{/if}

<style>
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    text-align: center;
    padding: 24px;
    pointer-events: none;
    color: var(--t-fg);
    animation: fade 200ms var(--ease) 120ms backwards;
  }
  .center p {
    margin: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    opacity: 0.6;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-top: 10px;
    pointer-events: auto;
  }
  .pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .banner {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    z-index: 5;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px 12px;
    max-width: calc(100% - 32px);
    padding: 8px 8px 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
    animation: up 160ms var(--ease);
  }
  .msg {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
  }
  .hint {
    padding-right: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes up {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
</style>
