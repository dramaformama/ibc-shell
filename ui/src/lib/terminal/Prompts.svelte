<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import type { Connection } from './connection.svelte';
  import PaneDialog from './PaneDialog.svelte';

  let {
    conn,
    link,
    paste,
    onlink,
    onpaste,
    onclosetab
  }: {
    conn: Connection;
    link: string | null;
    paste: { lines: number } | null;
    onlink: (open: boolean) => void;
    onpaste: (confirm: boolean) => void;
    onclosetab: () => void;
  } = $props();

  const prompt = $derived(conn.prompt);
  let answers = $state<string[]>([]);
  let passphrase = $state('');

  function submitKbd(e: SubmitEvent) {
    e.preventDefault();
    const a = answers;
    answers = [];
    void conn.answerKbd(a);
  }
  function cancelKbd() {
    answers = [];
    void conn.answerKbd(null);
  }
  function submitPassphrase(e: SubmitEvent) {
    e.preventDefault();
    const p = passphrase;
    passphrase = '';
    void conn.answerPassphrase(p);
  }
  function cancelPassphrase() {
    passphrase = '';
    void conn.answerPassphrase(null);
  }
</script>

{#if prompt?.kind === 'changed'}
  <PaneDialog title={m.term_hkc_title()} tone="danger">
    <p>{m.term_hkc_body({ host: prompt.mismatch.host, port: prompt.mismatch.port })}</p>
    <dl class="fp">
      <dt>{m.term_hkc_saved()}</dt>
      <dd>{prompt.mismatch.expected_fingerprint}</dd>
      <dt class="bad">{m.term_hkc_now()}</dt>
      <dd class="bad">{prompt.mismatch.actual_fingerprint}</dd>
    </dl>
    <p>{m.term_hkc_hint()}</p>
    {#snippet actions()}
      <button class="btn" onclick={onclosetab} data-autofocus>{m.term_close_tab()}</button>
    {/snippet}
  </PaneDialog>
{:else if prompt?.kind === 'kbd'}
  {#key prompt.pendingId}
    <PaneDialog title={prompt.name || m.term_kbd_title()} onescape={cancelKbd}>
      <form id="kbd-form" onsubmit={submitKbd}>
        {#if prompt.instruction}<p>{prompt.instruction}</p>{/if}
        {#each prompt.prompts as p, i (i)}
          <label class="field">
            <span>{p.text.replace(/[:：]\s*$/, '')}</span>
            <input
              class="input"
              type={p.echo ? 'text' : 'password'}
              autocomplete="off"
              spellcheck="false"
              autocapitalize="off"
              required
              data-autofocus={i === 0 ? '' : undefined}
              bind:value={answers[i]}
            />
          </label>
        {/each}
      </form>
      {#snippet actions()}
        <button class="btn btn-ghost" type="button" onclick={cancelKbd}>{m.action_cancel()}</button>
        <button class="btn btn-primary" type="submit" form="kbd-form">{m.term_kbd_submit()}</button>
      {/snippet}
    </PaneDialog>
  {/key}
{:else if prompt?.kind === 'passphrase'}
  {#key prompt.pendingId}
    <PaneDialog title={m.term_pp_title()} onescape={cancelPassphrase}>
      <form id="pp-form" onsubmit={submitPassphrase}>
        <p>{m.term_pp_body({ label: prompt.label })}</p>
        <label class="field">
          <span>{m.term_pp_field()}</span>
          <input
            class="input"
            type="password"
            autocomplete="off"
            required
            data-autofocus
            bind:value={passphrase}
          />
        </label>
      </form>
      {#snippet actions()}
        <button class="btn btn-ghost" type="button" onclick={cancelPassphrase}
          >{m.action_cancel()}</button
        >
        <button class="btn btn-primary" type="submit" form="pp-form">{m.term_kbd_submit()}</button>
      {/snippet}
    </PaneDialog>
  {/key}
{:else if link}
  <PaneDialog title={m.term_link_title()} onescape={() => onlink(false)}>
    <p>{m.term_link_body()}</p>
    <p class="url">{link}</p>
    {#snippet actions()}
      <button class="btn btn-ghost" onclick={() => onlink(false)}>{m.action_cancel()}</button>
      <button class="btn btn-primary" data-autofocus onclick={() => onlink(true)}
        >{m.term_link_open()}</button
      >
    {/snippet}
  </PaneDialog>
{:else if paste}
  <PaneDialog title={m.term_paste_title({ lines: paste.lines })} onescape={() => onpaste(false)}>
    <p>{m.term_paste_body()}</p>
    {#snippet actions()}
      <button class="btn btn-ghost" onclick={() => onpaste(false)}>{m.action_cancel()}</button>
      <button class="btn btn-primary" data-autofocus onclick={() => onpaste(true)}
        >{m.term_paste_confirm()}</button
      >
    {/snippet}
  </PaneDialog>
{/if}

<style>
  form {
    display: grid;
    gap: 12px;
  }
  .field {
    display: grid;
    gap: 6px;
    color: var(--text);
  }
  .field span {
    font-size: 12px;
    color: var(--muted);
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
  .fp .bad {
    color: var(--danger);
  }
  .url {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--r-control);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text);
  }
  .url {
    overflow-wrap: anywhere;
  }
</style>
