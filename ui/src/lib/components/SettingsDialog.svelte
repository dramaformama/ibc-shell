<script lang="ts">
  import { terminalPrefs } from '$lib/terminal/prefs.svelte';
  import { api, type AppInfo } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ACCENTS, settings, type ThemeChoice } from '$lib/stores/settings.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { AUTOLOCK_CHOICES } from '$lib/util/autolock';
  import { errorText } from '$lib/util/errors';
  import { MIN_PASSWORD, passwordStrength } from '$lib/util/strength';
  import Sheet from './Sheet.svelte';

  let info = $state<AppInfo | null>(null);
  let oldPw = $state('');
  let newPw = $state('');
  let msg = $state<{ text: string; ok: boolean } | null>(null);
  let busy = $state(false);

  $effect(() => {
    if (ui.settingsOpen) void api.appInfo().then((r) => r.status === 'ok' && (info = r.data));
  });

  const themes: { id: ThemeChoice; label: () => string }[] = [
    { id: 'light', label: m.theme_light },
    { id: 'dark', label: m.theme_dark },
    { id: 'system', label: m.theme_system }
  ];

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

{#if ui.settingsOpen}
  <Sheet title={m.settings_title()} onclose={() => (ui.settingsOpen = false)}>
    <div class="form">
      <section>
        <h3>{m.settings_appearance()}</h3>
        <div class="fld">
          <span class="lbl">{m.settings_theme()}</span>
          <div class="seg" role="group" aria-label={m.settings_theme()}>
            {#each themes as t (t.id)}
              <button
                type="button"
                aria-pressed={settings.theme === t.id}
                onclick={() => settings.setTheme(t.id)}>{t.label()}</button
              >
            {/each}
          </div>
        </div>
        <div class="fld">
          <span class="lbl">{m.settings_accent()}</span>
          <div class="swatches" role="radiogroup" aria-label={m.settings_accent()}>
            {#each ACCENTS as a (a)}
              <button
                type="button"
                role="radio"
                class="sw"
                data-accent-swatch={a}
                aria-checked={settings.accent === a}
                aria-label={a}
                title={a}
                onclick={() => settings.setAccent(a)}
              ></button>
            {/each}
          </div>
        </div>
        <div class="fld">
          <span class="lbl">{m.settings_language()}</span>
          <div class="seg" role="group" aria-label={m.settings_language()}>
            <button
              type="button"
              aria-pressed={settings.locale === 'en'}
              onclick={() => settings.setLanguage('en')}>English</button
            >
            <button
              type="button"
              aria-pressed={settings.locale === 'ru'}
              onclick={() => settings.setLanguage('ru')}>Русский</button
            >
          </div>
        </div>
      </section>

      <section>
        <h3>{m.settings_security()}</h3>
        <p class="note">
          {vault.mode === 'MasterPassword' ? m.vault_mode_password() : m.vault_mode_keychain()}
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
            {#if msg}<p class={msg.ok ? 'note' : 'error'} role="status">{msg.text}</p>{/if}
          </form>
        {/if}
        <label class="row">
          <span class="lbl">{m.settings_autolock()}</span>
          <select
            class="input"
            value={settings.autolockMinutes}
            onchange={(e) => settings.setAutolock(Number(e.currentTarget.value))}
          >
            {#each AUTOLOCK_CHOICES as n (n)}
              <option value={n}
                >{n === 0
                  ? m.settings_autolock_never()
                  : m.settings_autolock_minutes({ n })}</option
              >
            {/each}
          </select>
        </label>
        <button
          type="button"
          class="btn"
          onclick={() => ((ui.settingsOpen = false), void vault.lock())}>{m.action_lock()}</button
        >
      </section>

      <section>
        <h3>{m.settings_preferences()}</h3>
        <label class="row">
          <span class="lbl">{m.settings_highlight()}</span>
          <input
            type="checkbox"
            role="switch"
            checked={terminalPrefs.highlight}
            onchange={(e) => terminalPrefs.setHighlight(e.currentTarget.checked)}
          />
        </label>
        <label class="row">
          <span class="lbl">{m.settings_probe()}</span>
          <input
            type="checkbox"
            role="switch"
            checked={settings.probe}
            onchange={(e) => settings.setProbe(e.currentTarget.checked)}
          />
        </label>
        <p class="note">{m.settings_probe_note()}</p>
        <label class="row">
          <span class="lbl">{m.settings_update_check()}</span>
          <input
            type="checkbox"
            role="switch"
            checked={settings.updateCheck}
            onchange={(e) => settings.setUpdateCheck(e.currentTarget.checked)}
          />
        </label>
        <p class="note">{m.settings_update_note()}</p>
        {#if settings.update}
          {@const u = settings.update}
          <div class="row">
            <span class="lbl">{m.update_available({ version: u.version })}</span>
            <button type="button" class="btn" onclick={() => void api.openExternal(u.url)}
              >{m.update_open()}</button
            >
          </div>
        {/if}
      </section>
      {#if info}
        <p class="note tnum">{m.app_name()} {info.version} · {info.platform} {info.arch}</p>
      {/if}
    </div>
  </Sheet>
{/if}

<style>
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .sw {
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--sw);
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.1);
    transition: transform 120ms var(--ease);
  }
  .sw:hover {
    transform: scale(1.1);
  }
  .sw[aria-checked='true'] {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--sw);
  }
  .sw[data-accent-swatch='blue'] {
    --sw: #007aff;
  }
  .sw[data-accent-swatch='purple'] {
    --sw: #af52de;
  }
  .sw[data-accent-swatch='pink'] {
    --sw: #ff2d55;
  }
  .sw[data-accent-swatch='red'] {
    --sw: #ff3b30;
  }
  .sw[data-accent-swatch='orange'] {
    --sw: #ff9500;
  }
  .sw[data-accent-swatch='green'] {
    --sw: #34c759;
  }
  .sw[data-accent-swatch='teal'] {
    --sw: #30b0c7;
  }
  .sw[data-accent-swatch='indigo'] {
    --sw: #5856d6;
  }
  .sw[data-accent-swatch='graphite'] {
    --sw: #8e8e93;
  }
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
</style>
