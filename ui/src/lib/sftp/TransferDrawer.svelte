<script lang="ts">
  import { ChevronUp, Pause, Play, RotateCw, X } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { formatEta, formatSize, formatSpeed } from './model';
  import { etaOf, transfers, type TransferItem } from './transfers.svelte';

  const base = (p: string) => p.split(/[/\\]/).filter(Boolean).at(-1) ?? p;
  const pct = (t: TransferItem) => (t.total > 0 ? Math.min(100, (t.done / t.total) * 100) : 0);

  function status(t: TransferItem): string {
    switch (t.state) {
      case 'queued':
        return m.sftp_t_queued();
      case 'conflict':
        return m.sftp_t_conflict();
      case 'paused':
        return m.sftp_t_paused();
      case 'done':
        return t.skipped ? m.sftp_t_skipped() : formatSize(t.total);
      case 'failed':
        return t.error ?? m.sftp_t_failed();
      case 'cancelled':
        return m.sftp_t_cancelled();
      default:
        return [
          `${formatSize(t.done)} / ${formatSize(t.total)}`,
          formatSpeed(t.speed),
          formatEta(etaOf(t))
        ]
          .filter(Boolean)
          .join(' · ');
    }
  }
</script>

<div class="sf-drawer" class:open={transfers.open}>
  <div class="sf-dhead">
    <button
      class="sf-toggle"
      aria-expanded={transfers.open}
      aria-controls="sf-tlist"
      onclick={() => (transfers.open = !transfers.open)}
    >
      <ChevronUp size={14} class="chev" />
      {m.sftp_transfers()}
      {#if transfers.active > 0}<span class="badge tnum">{transfers.active}</span>{/if}
    </button>
    {#if transfers.open && transfers.finished > 0}
      <button class="btn btn-ghost sf-clear" onclick={() => transfers.clearFinished()}
        >{m.sftp_clear_finished()}</button
      >
    {/if}
  </div>
  {#if transfers.open}
    <ul id="sf-tlist" class="sf-tlist">
      {#each transfers.items as t (t.id)}
        <li class="sf-t" data-state={t.state}>
          <div class="top">
            <span class="nm" title={t.dstPath}>{base(t.dstPath)}</span>
            <span class="st tnum" class:err={t.state === 'failed'} title={t.error ?? undefined}
              >{status(t)}</span
            >
            <span class="ctl">
              {#if t.state === 'running'}
                <button
                  class="icon-btn"
                  title={m.sftp_pause()}
                  aria-label={m.sftp_pause()}
                  onclick={() => transfers.pause(t.id)}><Pause size={14} /></button
                >
              {:else if t.state === 'paused'}
                <button
                  class="icon-btn"
                  title={m.sftp_resume_t()}
                  aria-label={m.sftp_resume_t()}
                  onclick={() => transfers.resume(t.id)}><Play size={14} /></button
                >
              {:else if t.state === 'failed' || t.state === 'cancelled'}
                <button
                  class="icon-btn"
                  title={m.sftp_retry()}
                  aria-label={m.sftp_retry()}
                  onclick={() => transfers.retry(t.id)}><RotateCw size={14} /></button
                >
              {/if}
              {#if t.state === 'queued' || t.state === 'running' || t.state === 'paused' || t.state === 'conflict'}
                <button
                  class="icon-btn"
                  title={m.sftp_cancel()}
                  aria-label={m.sftp_cancel()}
                  onclick={() => transfers.cancel(t.id)}><X size={14} /></button
                >
              {/if}
            </span>
          </div>
          <div
            class="bar"
            role="progressbar"
            aria-label={base(t.dstPath)}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.round(pct(t))}
          >
            <i style:width="{t.state === 'done' ? 100 : pct(t)}%"></i>
          </div>
        </li>
      {:else}
        <li class="sf-tempty">{m.sftp_no_transfers()}</li>
      {/each}
    </ul>
  {/if}
</div>
