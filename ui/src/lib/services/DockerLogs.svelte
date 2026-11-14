<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { Copy, RotateCw, X } from '@lucide/svelte';
  import { tick } from 'svelte';
  import { api, type Container } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { colorLine, shortTime, stripAnsi } from './logcolor';

  /** Last log lines of one container, plain text (never HTML). */
  let { sid, container, onclose }: { sid: string; container: Container; onclose: () => void } =
    $props();

  const SIZES = [200, 500, 2000];
  let lines = $state(200);
  let text = $state<string | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let pre = $state<HTMLDivElement | null>(null);
  /**
   * Lines wrap like `journalctl`, so rows have no fixed height: they are grouped in blocks of 50
   * with `content-visibility: auto`, and the browser skips layout and paint of blocks off screen.
   */
  const BLOCK = 50;
  const parsed = $derived(
    text
      ? text
          .replace(/\n$/, '')
          .split('\n')
          .map((l) => colorLine(l))
      : []
  );
  const blocks = $derived(
    Array.from({ length: Math.ceil(parsed.length / BLOCK) }, (_, i) =>
      parsed.slice(i * BLOCK, (i + 1) * BLOCK)
    )
  );
  const locale = getLocale();

  async function load() {
    loading = true;
    error = null;
    const r = await api.dockerLogs(sid, container.id, lines).catch(() => null);
    loading = false;
    if (r?.status === 'ok') {
      text = r.data;
      await tick();
      pre?.scrollTo({ top: pre.scrollHeight });
    } else
      error =
        r?.status === 'error' && r.error.code === 'Validation'
          ? r.error.detail
          : m.dash_connection_error();
  }

  $effect(() => {
    void lines;
    void load();
  });

  async function copy() {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(stripAnsi(text));
      ui.toast(m.toast_copied());
    } catch {
      /* clipboard unavailable */
    }
  }
</script>

<Dialog.Root open onOpenChange={(o) => !o && onclose()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="logs-dialog">
      <header>
        <Dialog.Title>{m.dk_logs_title({ name: container.name })}</Dialog.Title>
        <div class="seg" role="radiogroup" aria-label={m.dk_lines()}>
          {#each SIZES as n (n)}
            <button
              type="button"
              role="radio"
              aria-checked={lines === n}
              onclick={() => (lines = n)}>{n}</button
            >
          {/each}
        </div>
        <button class="icon-btn" title={m.dk_refresh()} aria-label={m.dk_refresh()} onclick={load}
          ><RotateCw size={15} class={loading ? 'spin' : ''} /></button
        >
        <button
          class="icon-btn"
          title={m.dk_copy()}
          aria-label={m.dk_copy()}
          disabled={!text}
          onclick={copy}><Copy size={15} /></button
        >
        <Dialog.Close class="icon-btn" aria-label={m.action_close()}><X size={16} /></Dialog.Close>
      </header>
      {#if error}
        <p class="msg">{error}</p>
      {:else if text === null}
        <p class="msg">{m.dk_loading()}</p>
      {:else if !text.trim()}
        <p class="msg">{m.dk_empty_logs()}</p>
      {:else}
        <div class="log mono" bind:this={pre}>
          {#each blocks as block, b (b)}
            <div class="block">
              {#each block as line, i (b * BLOCK + i)}
                <div class="line">
                  {#if line.ts}<span class="ts" title={line.ts}>{shortTime(line.ts, locale)}</span
                    >{/if}{#each line.segs as seg, j (j)}<span class={seg.cls}>{seg.text}</span
                    >{/each}
                </div>
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.logs-dialog) {
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(980px, calc(100vw - 40px));
    height: min(720px, calc(100vh - 60px));
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 0.5px solid var(--border);
    border-radius: var(--r-sheet);
    background: var(--bg);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 12px 12px 18px;
    border-bottom: 0.5px solid var(--border);
  }
  header :global([data-dialog-title]) {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .seg {
    margin-right: 6px;
  }
  .log {
    flex: 1;
    padding: 12px 18px;
    overflow-x: hidden;
    overflow-y: auto;
    font-size: 12px;
    background: color-mix(in oklab, var(--surface) 60%, var(--bg));
    user-select: text;
  }
  .block {
    content-visibility: auto;
    contain-intrinsic-size: auto 950px;
  }
  /* Wrapped continuation lines are indented under the first one, as in journalctl. */
  .line {
    padding-left: 2ch;
    text-indent: -2ch;
    line-height: 19px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line .ts {
    margin-right: 1ch;
  }
  .ts,
  .dim {
    color: var(--muted);
  }
  .b {
    font-weight: 600;
  }
  .err,
  .a1,
  .a9 {
    color: var(--danger);
  }
  .warn,
  .a3,
  .a11 {
    color: color-mix(in oklab, var(--warn) 80%, var(--text));
  }
  .info,
  .a6,
  .a14 {
    color: var(--hl-cyan);
  }
  .ok,
  .a2,
  .a10 {
    color: var(--ok);
  }
  .dbg,
  .a0,
  .a8 {
    color: var(--muted);
  }
  .a4,
  .a12,
  .url {
    color: var(--hl-blue);
  }
  .ip,
  .a5,
  .a13 {
    color: var(--hl-magenta);
  }
  .num {
    color: var(--hl-orange);
  }
  .meth {
    color: var(--hl-orange);
    font-weight: 600;
  }
  .key {
    color: color-mix(in oklab, var(--hl-cyan) 70%, var(--muted));
  }
  .msg {
    margin: 0;
    padding: 24px 18px;
    color: var(--muted);
  }
  header :global(.spin) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      rotate: 360deg;
    }
  }
</style>
