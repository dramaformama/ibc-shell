<script lang="ts">
  import Logo from './Logo.svelte';
  import { Eye, EyeOff, KeyRound, Lock, ShieldCheck } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { vault } from '$lib/stores/vault.svelte';
  import { MIN_PASSWORD, passwordStrength } from '$lib/util/strength';
  import type { VaultMode } from '$lib/api';

  let mode = $state<VaultMode>('OsKeychain');
  let password = $state('');
  let confirm = $state('');
  let reveal = $state(false);
  let localError = $state<string | null>(null);

  const fresh = $derived(vault.state === 'Uninitialized');
  const needsPassword = $derived(
    fresh ? mode === 'MasterPassword' : vault.mode === 'MasterPassword'
  );
  const strength = $derived(passwordStrength(password));
  const strengthLabels = [
    m.strength_empty,
    m.strength_weak,
    m.strength_fair,
    m.strength_good,
    m.strength_strong
  ];

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    localError = null;
    if (fresh) {
      if (needsPassword) {
        if (password.length < MIN_PASSWORD)
          return (localError = m.err_password_short({ min: MIN_PASSWORD }));
        if (password !== confirm) return (localError = m.err_password_mismatch());
      }
      await vault.init(mode, needsPassword ? password : null);
    } else {
      await vault.unlock(needsPassword ? password : null);
    }
    password = '';
    confirm = '';
  }
  const focus = (n: HTMLElement) => n.focus();
</script>

<main class="screen">
  <div class="drag" data-tauri-drag-region></div>
  <form class="panel" onsubmit={submit} aria-labelledby="vault-title">
    <div class="mark" aria-hidden="true">
      <Logo size={56} />
      <span class="badge"
        >{#if fresh}<ShieldCheck size={13} />{:else}<Lock size={13} />{/if}</span
      >
    </div>
    <h1 id="vault-title">{fresh ? m.vault_welcome() : m.vault_locked_title()}</h1>
    <p class="lede">{fresh ? m.vault_welcome_hint() : m.vault_locked_hint()}</p>

    {#if fresh}
      <div class="modes" role="radiogroup" aria-label={m.vault_mode()}>
        <button
          type="button"
          role="radio"
          aria-checked={mode === 'OsKeychain'}
          class="mode"
          onclick={() => (mode = 'OsKeychain')}
        >
          <KeyRound size={16} />
          <span
            ><strong>{m.vault_mode_keychain()}</strong><small>{m.vault_mode_keychain_hint()}</small
            ></span
          >
        </button>
        <button
          type="button"
          role="radio"
          aria-checked={mode === 'MasterPassword'}
          class="mode"
          onclick={() => (mode = 'MasterPassword')}
        >
          <Lock size={16} />
          <span
            ><strong>{m.vault_mode_password()}</strong><small>{m.vault_mode_password_hint()}</small
            ></span
          >
        </button>
      </div>
    {/if}

    {#if needsPassword}
      <div class="pw">
        <input
          class="input"
          type={reveal ? 'text' : 'password'}
          bind:value={password}
          placeholder={m.field_master_password()}
          aria-label={m.field_master_password()}
          autocomplete={fresh ? 'new-password' : 'current-password'}
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
      {#if fresh}
        <input
          class="input"
          type={reveal ? 'text' : 'password'}
          bind:value={confirm}
          placeholder={m.field_confirm_password()}
          aria-label={m.field_confirm_password()}
          autocomplete="new-password"
        />
        <div class="meter" aria-live="polite">
          <div class="bars" data-level={strength}>
            {#each [1, 2, 3, 4] as n (n)}<i class:on={strength >= n}></i>{/each}
          </div>
          <span>{strengthLabels[strength]?.()}</span>
        </div>
      {/if}
    {/if}

    {#if localError || vault.error}<p class="error" role="alert">
        {localError ?? vault.error}
      </p>{/if}

    <!-- svelte-ignore a11y_autofocus -->
    <button
      type="submit"
      class="btn btn-primary go"
      disabled={vault.busy || (needsPassword && !password)}
      autofocus={!needsPassword}
    >
      {fresh ? m.vault_create() : m.vault_unlock()}
    </button>
  </form>
</main>

<style>
  .drag {
    position: fixed;
    inset: 0 0 auto;
    height: var(--toolbar-h);
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
  .modes {
    display: grid;
    gap: 8px;
    margin-bottom: 4px;
  }
  .mode {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      border-color 120ms var(--ease),
      background-color 120ms var(--ease);
  }
  .mode :global(svg) {
    margin-top: 2px;
    color: var(--muted);
  }
  .mode span {
    display: grid;
    gap: 2px;
  }
  .mode small {
    color: var(--muted);
    font-size: 12px;
  }
  .mode[aria-checked='true'] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .mode[aria-checked='true'] :global(svg) {
    color: var(--accent);
  }
  .pw {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .meter {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12px;
  }
  .bars {
    display: flex;
    gap: 4px;
    flex: 1;
  }
  .bars i {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--border);
    transition: background-color 150ms var(--ease);
  }
  .bars[data-level='1'] i.on {
    background: var(--danger);
  }
  .bars[data-level='2'] i.on {
    background: var(--warn);
  }
  .bars[data-level='3'] i.on,
  .bars[data-level='4'] i.on {
    background: var(--ok);
  }
  .go {
    height: 34px;
    margin-top: 4px;
  }
</style>
