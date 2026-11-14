<script lang="ts">
  import { Dialog } from 'bits-ui';
  import {
    ArrowDownToLine,
    Copy,
    ExternalLink,
    LoaderCircle,
    RefreshCw,
    Search
  } from '@lucide/svelte';
  import { api, type Endpoint, type Entry } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import { formatSize } from './model';

  let {
    entry,
    endpoint,
    path,
    transferLabel,
    close,
    transfer
  }: {
    entry: Entry;
    endpoint: Endpoint;
    path: string;
    transferLabel: string | null;
    close: () => void;
    transfer: () => void;
  } = $props();

  let preview = $state.raw<{ text: string; truncated: boolean } | null>(null);
  let tail = $state(false);
  let blocks = $state(1);
  let loading = $state(true);
  let previewError = $state('');
  let busy = $state(false);
  let openError = $state('');
  let unsupported = $state(false);
  let viewer = $state<HTMLTextAreaElement>();
  let search = $state<HTMLInputElement>();
  let query = $state('');
  let noMatch = $state(false);
  let wrap = $state(true);

  $effect(() => {
    const target = endpoint;
    const file = path;
    const fromEnd = tail;
    void loadPreview(target, file, fromEnd, blocks);
  });

  $effect(() => {
    if (viewer && tail) viewer.scrollTop = viewer.scrollHeight;
  });

  async function loadPreview(target: Endpoint, file: string, fromEnd: boolean, count: number) {
    loading = true;
    preview = null;
    previewError = '';
    noMatch = false;
    try {
      const result = await api.filePreview(target, file, fromEnd, count);
      if (result.status === 'ok') preview = result.data;
      else
        previewError =
          result.error.code === 'Unsupported'
            ? m.sftp_preview_unsupported()
            : errorText(result.error);
    } catch {
      previewError = m.sftp_open_failed();
    } finally {
      loading = false;
    }
  }

  function findNext() {
    if (!query || !viewer || !preview) return;
    const text = viewer.value;
    const pattern = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'giu');
    pattern.lastIndex = viewer.selectionEnd;
    const match = pattern.exec(text) ?? pattern.exec(text);
    noMatch = !match;
    if (!match) return;
    viewer.focus();
    viewer.setSelectionRange(match.index, match.index + match[0].length);
  }

  function keyboard(event: KeyboardEvent) {
    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f' || !search) return;
    event.preventDefault();
    event.stopPropagation();
    search.select();
    search.focus();
  }

  async function copyText() {
    if (!preview) return;
    try {
      await navigator.clipboard.writeText(preview.text);
      ui.toast(m.toast_copied());
    } catch {
      ui.toast(m.sftp_preview_copy_failed(), 'error');
    }
  }

  async function openFile() {
    if (busy) return;
    busy = true;
    openError = '';
    try {
      const result = await api.fileOpen(endpoint, path);
      if (result.status === 'ok') return;
      unsupported = result.error.code === 'Unsupported';
      openError = unsupported
        ? m.sftp_open_unsupported()
        : result.error.code === 'Internal'
          ? m.sftp_open_failed()
          : errorText(result.error);
    } catch {
      openError = m.sftp_open_failed();
    } finally {
      busy = false;
    }
  }
</script>

<Dialog.Root open onOpenChange={(open) => !open && close()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sf-dialog sf-preview" onkeydown={keyboard}>
      <header>
        <Dialog.Title>{entry.name}</Dialog.Title>
        <Dialog.Description class="preview-meta">
          <bdi>{path}</bdi>
          {#if entry.kind === 'file'}<span>{formatSize(entry.size)}</span>{/if}
        </Dialog.Description>
      </header>

      {#if preview?.text}
        <div class="preview-tools">
          <form
            onsubmit={(event) => {
              event.preventDefault();
              findNext();
            }}
          >
            <input
              class="input"
              type="search"
              bind:this={search}
              bind:value={query}
              oninput={() => (noMatch = false)}
              placeholder={m.sftp_preview_find()}
              aria-label={m.sftp_preview_find()}
            />
            <button
              class="btn"
              type="submit"
              disabled={!query}
              title={m.sftp_preview_next()}
              aria-label={m.sftp_preview_next()}
            >
              <Search size={15} />
            </button>
          </form>
          <button class="btn" aria-pressed={wrap} onclick={() => (wrap = !wrap)}>
            {m.sftp_preview_wrap()}
          </button>
          <button
            class="btn"
            onclick={copyText}
            title={m.sftp_preview_copy()}
            aria-label={m.sftp_preview_copy()}
          >
            <Copy size={15} />
          </button>
        </div>
        {#if noMatch}<div class="preview-meta" role="status">{m.sftp_preview_not_found()}</div>{/if}
        <textarea
          class="preview-text"
          readonly
          spellcheck={false}
          wrap={wrap ? 'soft' : 'off'}
          aria-label={entry.name}
          bind:this={viewer}
          value={preview.text}
          onkeydown={(event) => {
            if (event.key === 'F3') {
              event.preventDefault();
              findNext();
            }
          }}></textarea>
      {:else}
        <div class="preview-placeholder" role="status">
          {#if loading}
            <LoaderCircle class="spin" size={18} />{m.sftp_preview_loading()}
          {:else}
            {previewError || m.sftp_preview_empty()}
          {/if}
        </div>
      {/if}

      <div class="preview-tools">
        {#if preview?.truncated || tail}
          {#if preview?.truncated}
            <span class="preview-meta"
              >{tail
                ? m.sftp_preview_tail({ size: blocks * 5 })
                : m.sftp_preview_head({ size: blocks * 5 })}</span
            >
          {/if}
          <button
            class="btn"
            disabled={loading}
            onclick={() => {
              tail = !tail;
              blocks = 1;
            }}
          >
            {tail ? m.sftp_preview_show_head() : m.sftp_preview_show_tail()}
          </button>
        {/if}
        {#if preview?.truncated && blocks < 4}
          <button class="btn" onclick={() => (blocks += 1)}>{m.sftp_preview_more()}</button>
        {/if}
        <button
          class="btn refresh"
          disabled={loading}
          title={m.sftp_preview_refresh()}
          aria-label={m.sftp_preview_refresh()}
          onclick={() => void loadPreview(endpoint, path, tail, blocks)}
          ><RefreshCw size={15} /></button
        >
      </div>

      {#if openError}<p class="open-error" role="alert">{openError}</p>{/if}
      <div class="sf-actions sf-choices">
        <Dialog.Close class="btn btn-ghost">{m.action_close()}</Dialog.Close>
        {#if transferLabel}
          <button class="btn" onclick={transfer}>
            <ArrowDownToLine size={14} />{transferLabel}
          </button>
        {/if}
        <button class="btn" disabled={busy || unsupported} onclick={openFile}>
          {#if busy}<LoaderCircle class="spin" size={14} />{m.sftp_opening_file()}
          {:else}<ExternalLink size={14} />{m.sftp_open_system()}{/if}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sf-dialog.sf-preview) {
    width: min(860px, calc(100vw - 32px));
    height: min(690px, calc(100dvh - 40px));
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  header {
    min-width: 0;
  }
  header :global(h2) {
    overflow-wrap: anywhere;
  }
  .preview-meta,
  :global(.sf-preview .preview-meta) {
    font-size: 12px;
    color: var(--muted);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 12px;
  }
  bdi {
    overflow-wrap: anywhere;
  }
  .preview-tools,
  .preview-tools form {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  form {
    flex: 1;
    min-width: min(230px, 100%);
  }
  input {
    flex: 1;
    min-width: 0;
  }
  .preview-text,
  .preview-placeholder {
    flex: 1;
    min-height: 100px;
    border: 1px solid var(--border);
    border-radius: var(--r-control);
    background: var(--surface);
  }
  .preview-text {
    padding: 12px;
    resize: none;
    font:
      13px/1.55 ui-monospace,
      monospace;
    tab-size: 4;
    color: inherit;
    direction: ltr;
  }
  .preview-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
    text-align: center;
    color: var(--muted);
  }
  .refresh {
    margin-inline-start: auto;
  }
  .open-error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
