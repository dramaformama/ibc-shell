<script lang="ts">
  import { api } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { errorText } from '$lib/util/errors';
  import { restoreLocal, saveLocal } from '$lib/util/localprefs';

  /** Settings section: one encrypted file with everything, and restoring from it. */
  const MIN = 10;
  let savePw = $state('');
  let savePw2 = $state('');
  let restorePw = $state('');
  let busy = $state<'save' | 'restore' | null>(null);
  let msg = $state<{ text: string; ok: boolean } | null>(null);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    msg = null;
    if (savePw.length < MIN) return (msg = { text: m.err_password_short({ min: MIN }), ok: false });
    if (savePw !== savePw2) return (msg = { text: m.err_password_mismatch(), ok: false });
    busy = 'save';
    await saveLocal();
    const r = await api.vaultExport(savePw);
    busy = null;
    if (r.status === 'error') return (msg = { text: errorText(r.error), ok: false });
    if (r.data) {
      savePw = savePw2 = '';
      msg = { text: m.backup_saved(), ok: true };
    }
  }

  async function restore(e: SubmitEvent) {
    e.preventDefault();
    msg = null;
    busy = 'restore';
    const r = await api.vaultImport(restorePw);
    busy = null;
    if (r.status === 'error') return (msg = { text: errorText(r.error), ok: false });
    const s = r.data;
    if (!s) return; // the file dialog was cancelled
    restorePw = '';
    await restoreLocal();
    msg = {
      text: [m.backup_restored(s), s.skipped ? m.backup_skipped({ skipped: s.skipped }) : '']
        .join(' ')
        .trim(),
      ok: true
    };
    // Theme, language and layout are read at start-up.
    setTimeout(() => location.reload(), 1800);
  }
</script>

<section>
  <h3>{m.backup_title()}</h3>
  <p class="note">{m.backup_note()}</p>

  <form class="bk" onsubmit={save}>
    <b class="sub">{m.backup_save_h()}</b>
    <input
      class="input"
      type="password"
      bind:value={savePw}
      placeholder={m.backup_pw()}
      aria-label={m.backup_pw()}
      autocomplete="new-password"
    />
    <input
      class="input"
      type="password"
      bind:value={savePw2}
      placeholder={m.backup_pw2()}
      aria-label={m.backup_pw2()}
      autocomplete="new-password"
    />
    <p class="note">{m.backup_pw_hint({ min: MIN })}</p>
    <button class="btn btn-primary" disabled={busy !== null || !savePw || !savePw2}
      >{m.backup_save()}</button
    >
  </form>

  <form class="bk" onsubmit={restore}>
    <b class="sub">{m.backup_restore_h()}</b>
    <p class="note">{m.backup_restore_note()}</p>
    <input
      class="input"
      type="password"
      bind:value={restorePw}
      placeholder={m.backup_restore_pw()}
      aria-label={m.backup_restore_pw()}
      autocomplete="off"
    />
    <button class="btn" disabled={busy !== null || !restorePw}>{m.backup_restore()}</button>
  </form>

  {#if msg}<p class={msg.ok ? 'note' : 'error'} role="status">{msg.text}</p>{/if}
</section>

<style>
  .bk {
    display: grid;
    gap: 8px;
  }
  .bk + .bk {
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .sub {
    font-size: 12px;
    font-weight: 600;
  }
  .btn {
    justify-self: start;
  }
</style>
