<script lang="ts">
  import { api } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { settings } from '$lib/stores/settings.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { AUTOLOCK_CHOICES } from '$lib/util/autolock';
  import { errorText } from '$lib/util/errors';
  import { MIN_PASSWORD, passwordStrength } from '$lib/util/strength';
  import BackupSection from './BackupSection.svelte';

  /** Settings → Security: master password, auto-lock, backup. */
  let oldPw = $state('');
  let newPw = $state('');
  let msg = $state<{ text: string; ok: boolean } | null>(null);
  let busy = $state(false);

  async function changePassword(e: SubmitEvent) {
    e.preventDefault();
    msg = null;
    if (newPw.length < MIN_PASSWORD)
      return (msg = { text: m.err_password_short({ min: MIN_PASSWORD }), ok: false });
    busy = true;
    const r = await api.vaultChangePassword(oldPw, newPw);
    busy = false;
    oldPw = newPw = '';
    msg =
      r.status === 'ok'
        ? { text: m.settings_password_changed(), ok: true }
        : { text: errorText(r.error), ok: false };
  }
</script>

<section>
  <h3>{m.settings_security()}</h3>
  <p class="note">
    {vault.mode === 'MasterPassword' ? m.vault_mode_password() : m.settings_keychain_note()}
  </p>
  {#if vault.mode === 'MasterPassword'}
    <form class="pwform" onsubmit={changePassword}>
      <input
        class="input"
        type="password"
        bind:value={oldPw}
        placeholder={m.settings_current_password()}
        aria-label={m.settings_current_password()}
        autocomplete="current-password"
      />
      <input
        class="input"
        type="password"
        bind:value={newPw}
        placeholder={m.settings_new_password()}
        aria-label={m.settings_new_password()}
        autocomplete="new-password"
      />
      <div class="row">
        <span class="note"
          >{[
            m.strength_empty,
            m.strength_weak,
            m.strength_fair,
            m.strength_good,
            m.strength_strong
          ][passwordStrength(newPw)]?.()}</span
        >
        <button class="btn" disabled={busy || !oldPw || !newPw}
          >{m.settings_change_password()}</button
        >
      </div>
      {#if msg}<p class={msg.ok ? 'note' : 'error'} role="status">
          {msg.text}
        </p>{/if}
    </form>
  {/if}
  {#if vault.canLock}
    <label class="row">
      <span class="lbl">{m.settings_autolock()}</span>
      <select
        class="input"
        value={settings.autolockMinutes}
        onchange={(e) => settings.setAutolock(Number(e.currentTarget.value))}
      >
        {#each AUTOLOCK_CHOICES as n (n)}
          <option value={n}
            >{n === 0 ? m.settings_autolock_never() : m.settings_autolock_minutes({ n })}</option
          >
        {/each}
      </select>
    </label>
    <button type="button" class="btn" onclick={() => ((ui.settingsOpen = false), void vault.lock())}
      >{m.action_lock()}</button
    >
  {/if}
</section>
<BackupSection />

<style>
  .pwform {
    display: grid;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .btn {
    justify-self: start;
  }
  .row .lbl {
    color: var(--text);
  }
  .row select {
    width: auto;
    min-width: 140px;
  }
</style>
