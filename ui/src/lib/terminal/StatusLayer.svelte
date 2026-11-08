<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import ConnectCard from './ConnectCard.svelte';
  import type { Connection } from './connection.svelte';

  let { conn, name, onclosetab }: { conn: Connection; name: string; onclosetab: () => void } =
    $props();

  /** Localized reason of a dropped connection (codes from the backend), else the raw text. */
  function endText(r: string | null): string | null {
    if (!r || r === 'closed') return null;
    if (r === 'shell_exited') return m.term_end_shell_exited();
    if (r === 'keepalive') return m.term_end_keepalive();
    if (r === 'inactivity') return m.term_end_inactivity();
    if (r === 'protocol') return m.term_end_protocol();
    if (r.startsWith('server:')) {
      const code = r.slice(7);
      if (code === 'ByApplication') return m.term_end_server_ByApplication();
      if (code === 'TooManyConnections') return m.term_end_server_TooManyConnections();
      if (code === 'ConnectionLost') return m.term_end_server_ConnectionLost();
      return m.term_end_server({ code });
    }
    if (r.startsWith('network:')) {
      const kind = r.slice(8);
      if (kind === 'ConnectionReset' || kind === 'ConnectionAborted' || kind === 'BrokenPipe')
        return m.term_end_reset();
      if (kind === 'TimedOut') return m.term_end_timeout();
      return m.term_end_network({ kind });
    }
    return r;
  }
  const endReason = $derived(endText(conn.reason));

  const blocked = $derived(conn.prompt?.kind === 'changed');
</script>

{#if (conn.phase === 'connecting' && !conn.prompt) || (conn.phase === 'failed' && !blocked)}
  <div class="center">
    <ConnectCard
      log={conn.log}
      {name}
      failed={conn.phase === 'failed'}
      {onclosetab}
      onretry={() => void conn.retry()}
    />
  </div>
{:else if conn.phase === 'disconnected'}
  <div class="banner" role="status">
    <span class="msg">
      <span class="dot" aria-hidden="true"></span>
      {endReason ? `${m.term_disconnected()} · ${endReason}` : m.term_disconnected()}
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
    place-items: center;
    padding: 24px;
    pointer-events: none;
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
  @keyframes up {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }
</style>
