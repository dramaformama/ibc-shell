<script lang="ts">
  import { Dialog } from 'bits-ui';
  import type { ConflictChoice } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { formatDate, formatSize } from './model';
  import { transfers } from './transfers.svelte';

  const cur = $derived(transfers.conflicts[0] ?? null);
  const more = $derived(Math.max(0, transfers.conflicts.length - 1));
  let all = $state(false);

  const name = $derived(cur?.info.path.split(/[/\\]/).filter(Boolean).at(-1) ?? '');
  const newer = $derived(
    cur && cur.info.src_mtime !== null && cur.info.dst_mtime !== null
      ? cur.info.src_mtime > cur.info.dst_mtime
      : null
  );
  const canResume = $derived(
    !!cur && cur.info.dst_size > 0 && cur.info.dst_size < cur.info.src_size
  );

  function pick(c: ConflictChoice) {
    if (!cur) return;
    const a = all;
    all = false;
    void transfers.reply(cur, c, a);
  }
</script>

<!-- Not dismissible: every conflict needs an explicit answer (Skip is the safe one). -->
<Dialog.Root open={cur !== null}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content
      class="sf-dialog sf-conflict"
      interactOutsideBehavior="ignore"
      escapeKeydownBehavior="ignore"
    >
      {#if cur}
        <Dialog.Title>{m.sftp_conflict_title()}</Dialog.Title>
        <Dialog.Description class="sf-name" title={cur.info.path}>{name}</Dialog.Description>
        <div class="sf-compare">
          <div>
            <h3>{m.sftp_conflict_source()}</h3>
            <p class="tnum">{formatSize(cur.info.src_size)}</p>
            <p class="tnum muted">{formatDate(cur.info.src_mtime)}</p>
            {#if newer === true}<span class="tag">{m.sftp_conflict_newer()}</span>{/if}
          </div>
          <div>
            <h3>{m.sftp_conflict_existing()}</h3>
            <p class="tnum">{formatSize(cur.info.dst_size)}</p>
            <p class="tnum muted">{formatDate(cur.info.dst_mtime)}</p>
            {#if newer === false}<span class="tag">{m.sftp_conflict_newer()}</span>{/if}
          </div>
        </div>
        <label class="sf-check">
          <input type="checkbox" bind:checked={all} />
          {more > 0 ? m.sftp_conflict_all_n({ count: more }) : m.sftp_conflict_all()}
        </label>
        <div class="sf-actions sf-choices">
          <button class="btn" onclick={() => pick('Skip')}>{m.sftp_skip()}</button>
          <button class="btn" onclick={() => pick('Rename')}>{m.sftp_rename_keep()}</button>
          <button class="btn" disabled={!canResume} onclick={() => pick('Resume')}
            >{m.sftp_resume()}</button
          >
          <button class="btn btn-primary" onclick={() => pick('Overwrite')}
            >{m.sftp_overwrite()}</button
          >
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
