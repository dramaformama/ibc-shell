<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { api } from '$lib/api';
  import { ACCENTS, settings, type ThemeChoice } from '$lib/stores/settings.svelte';

  /** Settings → General: appearance and app behaviour. */
  const themes: { id: ThemeChoice; label: () => string }[] = [
    { id: 'light', label: m.theme_light },
    { id: 'dark', label: m.theme_dark },
    { id: 'system', label: m.theme_system }
  ];
</script>

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
    <span class="lbl">{m.settings_density()}</span>
    <div class="seg" role="group" aria-label={m.settings_density()}>
      <button
        type="button"
        aria-pressed={settings.density === 'roomy'}
        onclick={() => settings.setDensity('roomy')}>{m.density_roomy()}</button
      >
      <button
        type="button"
        aria-pressed={settings.density === 'compact'}
        onclick={() => settings.setDensity('compact')}>{m.density_compact()}</button
      >
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
  <h3>{m.settings_preferences()}</h3>
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
    <span class="lbl">{m.settings_tray()}</span>
    <input
      type="checkbox"
      role="switch"
      checked={settings.tray}
      onchange={(e) => settings.setTray(e.currentTarget.checked)}
    />
  </label>
  <p class="note">{m.settings_tray_note()}</p>
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
</style>
