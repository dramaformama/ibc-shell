<script lang="ts">
  import { Dialog } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { openConnections } from './connection.svelte';

  /**
   * First-contact host key question for any open connection, shown over the whole app. Each
   * key is confirmed on its own (one question at a time) — there is no "accept all".
   */
  const waiting = $derived([...openConnections].filter((c) => c.prompt?.kind === 'hostkey'));
  const conn = $derived(waiting[0] ?? null);
  const prompt = $derived(conn?.prompt?.kind === 'hostkey' ? conn.prompt : null);
</script>

<Dialog.Root open={prompt !== null} onOpenChange={(o) => !o && void conn?.rejectHostKey()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content
      class="confirm hostkey"
      interactOutsideBehavior="ignore"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        document.querySelector<HTMLElement>('.hostkey [data-autofocus]')?.focus();
      }}
    >
      {#if prompt && conn}
        <Dialog.Title>{m.term_hk_title()}</Dialog.Title>
        <Dialog.Description>
          {m.term_hk_body({ host: prompt.info.host, port: prompt.info.port })}
        </Dialog.Description>
        <p class="who">
          <b>{conn.label}</b>{#if waiting.length > 1}<span class="n tnum"
              >{m.term_hk_more({ count: waiting.length - 1 })}</span
            >{/if}
        </p>
        <dl class="fp">
          <dt>{m.term_hk_type()}</dt>
          <dd>{prompt.info.key_type}</dd>
          <dt>{m.term_hk_fp()}</dt>
          <dd>{prompt.info.fingerprint}</dd>
        </dl>
        <pre class="art" aria-label={m.term_hk_art()}>{prompt.info.randomart}</pre>
        <div class="actions">
          <button class="btn btn-ghost" onclick={() => void conn.rejectHostKey()}
            >{m.action_cancel()}</button
          >
          <button class="btn" onclick={() => void conn.acceptHostKey(false)}
            >{m.term_hk_once()}</button
          >
          <button
            class="btn btn-primary"
            data-autofocus
            onclick={() => void conn.acceptHostKey(true)}>{m.term_hk_save()}</button
          >
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.confirm.hostkey) {
    width: min(460px, calc(100vw - 32px));
  }
  .who {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0;
  }
  .n {
    color: var(--muted);
    font-size: 12px;
  }
  .fp {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 6px 14px;
    margin: 0;
    font-size: 12px;
  }
  .fp dt {
    color: var(--muted);
  }
  .fp dd {
    margin: 0;
    font-family: var(--font-mono);
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .art {
    justify-self: start;
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--r-control);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.25;
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }
</style>
