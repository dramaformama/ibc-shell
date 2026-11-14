<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { ArrowDownToLine, ExternalLink, File, LoaderCircle } from '@lucide/svelte';
  import { api, type Endpoint, type Entry } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
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

  let busy = $state(false);
  let error = $state('');
  let unsupported = $state(false);

  function dismiss() {
    if (!busy) close();
  }

  async function openFile() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const result = await api.fileOpen(endpoint, path);
      if (result.status === 'ok') close();
      else {
        unsupported = result.error.code === 'Unsupported';
        error = unsupported
          ? m.sftp_open_unsupported()
          : result.error.code === 'Internal'
            ? m.sftp_open_failed()
            : errorText(result.error);
      }
    } catch {
      error = m.sftp_open_failed();
    } finally {
      busy = false;
    }
  }
</script>

<Dialog.Root open onOpenChange={(open) => !open && dismiss()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sf-dialog sf-open">
      <div class="heading">
        <span class="file-icon" aria-hidden="true"><File size={19} /></span>
        <div class="file-label">
          <Dialog.Title>{entry.name}</Dialog.Title>
          {#if entry.kind === 'file'}<span class="size">{formatSize(entry.size)}</span>{/if}
        </div>
      </div>
      <Dialog.Description>
        {endpoint.kind === 'remote' ? m.sftp_open_remote_hint() : m.sftp_open_local_hint()}
      </Dialog.Description>
      {#if error}<p class="open-error" role="alert">{error}</p>{/if}
      <div class="sf-actions sf-choices">
        <Dialog.Close class="btn btn-ghost" disabled={busy}>{m.action_cancel()}</Dialog.Close>
        {#if transferLabel}
          <button class="btn" disabled={busy} onclick={transfer}>
            <ArrowDownToLine size={14} />{transferLabel}
          </button>
        {/if}
        <button class="btn btn-primary" disabled={busy || unsupported} onclick={openFile}>
          {#if busy}<LoaderCircle class="spin" size={14} />{m.sftp_opening_file()}
          {:else}<ExternalLink size={14} />{m.sftp_menu_open()}{/if}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sf-dialog.sf-open) {
    width: min(440px, calc(100vw - 32px));
  }
  .heading {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .file-icon {
    flex: none;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: var(--r-control);
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, var(--surface));
  }
  .file-label {
    min-width: 0;
  }
  .size {
    font-size: 12px;
    color: var(--muted);
  }
  .open-error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
