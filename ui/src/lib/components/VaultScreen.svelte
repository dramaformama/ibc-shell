<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core';
  import { isApple } from '$lib/util/platform';
  import WindowControls from './WindowControls.svelte';
  import Logo from './Logo.svelte';
  import { Eye, EyeOff, KeyRound, Loader } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { vault } from '$lib/stores/vault.svelte';

  const windowControls = isTauri() && !isApple && !/Android/.test(navigator.userAgent);

  let password = $state('');
  let reveal = $state(false);

  /** A vault from before ADR-014: asks for the old password once, then moves it to the keychain. */
  const legacy = $derived(vault.mode === 'MasterPassword');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    await vault.removePassword(password);
    password = '';
  }
  async function justOpen() {
    await vault.unlock(password);
    password = '';
  }
  function retry() {
    vault.error = null;
    void vault.auto();
  }
  const focus = (n: HTMLElement) => n.focus();
</script>

<main class="screen">
  <div class="drag" data-tauri-drag-region>
    {#if windowControls}<WindowControls />{/if}
  </div>
  <form class="panel" onsubmit={submit} aria-labelledby="vault-title">
    <div class="mark" aria-hidden="true">
      <Logo size={56} />
      <span class="badge"><KeyRound size={13} /></span>
    </div>

    {#if legacy}
      <h1 id="vault-title">{m.vault_legacy_title()}</h1>
      <p class="lede">{m.vault_legacy_hint()}</p>
      <div class="pw">
        <input
          class="input"
          type={reveal ? 'text' : 'password'}
          bind:value={password}
          placeholder={m.field_master_password()}
          aria-label={m.field_master_password()}
          autocomplete="current-password"
          use:focus
        />
        <button
          type="button"
          class="icon-btn"
          aria-label={reveal ? m.action_hide() : m.action_show()}
          onclick={() => (reveal = !reveal)}
        >
          {#if reveal}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
        </button>
      </div>
      {#if vault.error}<p class="error" role="alert">{vault.error}</p>{/if}
      <button type="submit" class="btn btn-primary go" disabled={vault.busy || !password}>
        {m.vault_legacy_go()}
      </button>
      <button type="button" class="btn go" disabled={vault.busy || !password} onclick={justOpen}>
        {m.vault_legacy_only_unlock()}
      </button>
    {:else if vault.error}
      <h1 id="vault-title">{m.vault_open_failed()}</h1>
      <p class="error" role="alert">{vault.error}</p>
      <p class="quiet">{m.vault_open_failed_hint()}</p>
      <button type="button" class="btn btn-primary go" onclick={retry}>{m.vault_retry()}</button>
    {:else}
      <h1 id="vault-title" class="wait"><Loader size={16} />{m.vault_opening()}</h1>
    {/if}
  </form>
</main>

<style>
  .drag {
    position: fixed;
    inset: 0 0 auto;
    height: var(--toolbar-h);
    display: flex;
    justify-content: flex-end;
    align-items: center;
    padding-inline: 8px;
  }
  .screen {
    height: 100%;
    display: grid;
    place-items: center;
    padding: 24px 16px;
    overflow: auto;
  }
  .panel {
    width: min(360px, 100%);
    display: grid;
    gap: 12px;
  }
  .mark {
    position: relative;
    width: 56px;
    height: 56px;
    margin-bottom: 6px;
    filter: drop-shadow(0 6px 14px oklch(0 0 0 / 0.18));
  }
  .badge {
    position: absolute;
    right: -6px;
    bottom: -6px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--surface);
    color: var(--accent);
    box-shadow: 0 0 0 0.5px var(--border);
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .lede {
    margin: -4px 0 8px;
    color: var(--muted);
    text-wrap: pretty;
  }
  .pw {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .go {
    height: 34px;
    margin-top: 4px;
  }
  .quiet {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    text-wrap: pretty;
  }
  .wait {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .wait :global(svg) {
    animation: turn 1s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .wait :global(svg) {
      animation: none;
    }
  }
</style>
