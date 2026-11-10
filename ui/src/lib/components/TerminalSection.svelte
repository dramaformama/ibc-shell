<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import {
    FONT_MAX,
    FONT_MIN,
    SCROLLBACK_CHOICES,
    terminalPrefs
  } from '$lib/terminal/prefs.svelte';
  import { TERMINAL_THEMES } from '$lib/terminal/themes';
  import { ui } from '$lib/stores/ui.svelte';
  import { TERM_THEMES } from '$lib/util/defaults-form';
  import { Minus, Plus } from '@lucide/svelte';

  void terminalPrefs.load();

  let el = $state<HTMLElement>();
  $effect(() => {
    if (el && ui.settingsFocus === 'terminal') {
      el.scrollIntoView({ block: 'start' });
      ui.settingsFocus = null;
    }
  });

  const swatches = ['red', 'green', 'yellow', 'blue', 'magenta', 'cyan'] as const;
</script>

<section bind:this={el}>
  <h3>{m.settings_terminal()}</h3>
  <div class="fld">
    <span class="lbl">{m.field_term_theme()}</span>
    <div class="themes" role="radiogroup" aria-label={m.field_term_theme()}>
      <button
        type="button"
        role="radio"
        class="card"
        aria-checked={terminalPrefs.theme === null}
        onclick={() => terminalPrefs.setTheme(null)}
      >
        <span class="pv auto"><span class="aa">Aa</span></span>
        <span class="name">{m.settings_term_auto()}</span>
      </button>
      {#each TERM_THEMES as t (t.id)}
        {@const c = TERMINAL_THEMES[t.id]!}
        <button
          type="button"
          role="radio"
          class="card"
          aria-checked={terminalPrefs.theme === t.id}
          onclick={() => terminalPrefs.setTheme(t.id)}
        >
          <span class="pv" style:background={c.background} style:color={c.foreground}>
            <span class="aa">Aa</span>
            <span class="dots">
              {#each swatches as s (s)}<i style:background={c[s]}></i>{/each}
            </span>
          </span>
          <span class="name">{t.name}</span>
        </button>
      {/each}
    </div>
    <p class="note">{m.settings_term_theme_note()}</p>
  </div>

  <div class="row">
    <span class="lbl">{m.settings_term_font()}</span>
    <div class="stepper">
      <button
        type="button"
        class="btn btn-ghost"
        aria-label={m.settings_term_font_smaller()}
        title={m.settings_term_font_smaller()}
        disabled={terminalPrefs.fontSize <= FONT_MIN}
        onclick={() => terminalPrefs.zoom(-1)}><Minus size={14} /></button
      >
      <output class="tnum">{terminalPrefs.fontSize}</output>
      <button
        type="button"
        class="btn btn-ghost"
        aria-label={m.settings_term_font_larger()}
        title={m.settings_term_font_larger()}
        disabled={terminalPrefs.fontSize >= FONT_MAX}
        onclick={() => terminalPrefs.zoom(1)}><Plus size={14} /></button
      >
    </div>
  </div>

  <label class="row">
    <span class="lbl">{m.settings_term_scrollback()}</span>
    <select
      class="input short"
      value={terminalPrefs.scrollback}
      onchange={(e) => terminalPrefs.setScrollback(Number(e.currentTarget.value))}
    >
      {#each SCROLLBACK_CHOICES as n (n)}
        <option value={n}>{n.toLocaleString()}</option>
      {/each}
    </select>
  </label>

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
    <span class="lbl">{m.settings_notify()}</span>
    <input
      type="checkbox"
      role="switch"
      checked={terminalPrefs.notify}
      onchange={(e) => terminalPrefs.setNotify(e.currentTarget.checked)}
    />
  </label>
  <p class="note">{m.settings_notify_note()}</p>
</section>

<style>
  section {
    scroll-margin-top: 8px;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 8px;
    margin-top: 4px;
  }
  .card {
    display: grid;
    gap: 4px;
    padding: 4px;
    border: 0;
    border-radius: var(--r-control);
    background: transparent;
    color: var(--text);
    font: inherit;
    cursor: pointer;
    text-align: start;
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .card:hover {
    background: var(--hover);
  }
  .card[aria-checked='true'] {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .pv {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 4px;
    height: 44px;
    padding: 6px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px rgb(127 127 127 / 0.2);
  }
  .pv.auto {
    background: linear-gradient(135deg, #ffffff 50%, #0e0f12 50%);
    color: #16181d;
  }
  .aa {
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1;
  }
  .dots {
    display: grid;
    grid-template-columns: repeat(3, 6px);
    gap: 2px;
  }
  .dots i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .name {
    overflow: hidden;
    padding: 0 2px 2px;
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .stepper .btn {
    width: var(--control-h);
    padding: 0;
    justify-content: center;
  }
  output {
    min-width: 26px;
    text-align: center;
  }
  .short {
    width: 120px;
  }
</style>
