<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { KeyRound, Lock, ServerCog, SlidersHorizontal, SquareTerminal, X } from '@lucide/svelte';
  import { api, type AppInfo } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import GeneralPane from './GeneralPane.svelte';
  import SecurityPane from './SecurityPane.svelte';
  import KeysSection from './KeysSection.svelte';
  import KnownHostsSection from './KnownHostsSection.svelte';
  import TerminalSection from './TerminalSection.svelte';

  /** Settings as a window of its own (macOS Settings): a tab toolbar, one pane at a time. */
  type Pane = 'general' | 'terminal' | 'keys' | 'known' | 'security';
  const panes = [
    { id: 'general' as const, label: m.section_general, icon: SlidersHorizontal },
    { id: 'terminal' as const, label: m.settings_terminal, icon: SquareTerminal },
    { id: 'keys' as const, label: m.settings_keys, icon: KeyRound },
    { id: 'known' as const, label: m.settings_known, icon: ServerCog },
    { id: 'security' as const, label: m.settings_security, icon: Lock }
  ];
  let pane = $state<Pane>('general');
  $effect(() => {
    if (!ui.settingsOpen) return;
    const f = ui.settingsFocus;
    pane =
      f === 'terminal'
        ? 'terminal'
        : f === 'known-hosts'
          ? 'known'
          : f === 'backup'
            ? 'security'
            : pane;
    if (f === 'backup') ui.settingsFocus = null;
  });

  let info = $state<AppInfo | null>(null);
  $effect(() => {
    if (ui.settingsOpen) void api.appInfo().then((r) => r.status === 'ok' && (info = r.data));
  });
</script>

{#if ui.settingsOpen}
  <Dialog.Root open onOpenChange={(o) => !o && (ui.settingsOpen = false)}>
    <Dialog.Portal>
      <Dialog.Overlay class="overlay" />
      <Dialog.Content
        class="prefs"
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          document.querySelector<HTMLElement>('.prefs [role="tab"][aria-selected="true"]')?.focus();
        }}
      >
        <header>
          <Dialog.Title>{panes.find((p) => p.id === pane)?.label()}</Dialog.Title>
          <Dialog.Close class="icon-btn close" aria-label={m.action_close()}
            ><X size={15} /></Dialog.Close
          >
          <div class="tabs" role="tablist" aria-label={m.settings_title()}>
            {#each panes as p (p.id)}
              <button role="tab" aria-selected={pane === p.id} onclick={() => (pane = p.id)}
                ><p.icon size={20} strokeWidth={1.5} /><span>{p.label()}</span></button
              >
            {/each}
          </div>
        </header>
        <div class="body" role="tabpanel">
          {#key pane}
            <div class="form pane">
              {#if pane === 'general'}
                <GeneralPane />
              {:else if pane === 'terminal'}
                <TerminalSection />
              {:else if pane === 'keys'}
                <KeysSection />
              {:else if pane === 'known'}
                <KnownHostsSection />
              {:else}
                <SecurityPane />
              {/if}
              {#if info}
                <p class="note tnum">{m.app_name()} {info.version} · {info.platform} {info.arch}</p>
              {/if}
            </div>
          {/key}
        </div>
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
{/if}

<style>
  :global(.prefs) {
    position: fixed;
    z-index: 51;
    top: 50%;
    left: 50%;
    width: min(660px, calc(100vw - 24px));
    height: min(620px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 0.5px solid var(--glass-edge);
    border-radius: var(--r-sheet);
    background: var(--bg);
    box-shadow: var(--shadow);
    transform: translate(-50%, -50%);
    animation: prefs-in 380ms var(--spring);
  }
  @keyframes prefs-in {
    from {
      opacity: 0;
      transform: translate(-50%, -48%) scale(0.96);
    }
  }
  header {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 12px 12px 8px;
    border-bottom: 0.5px solid var(--sep);
    background: var(--glass-strong);
  }
  header :global([data-dialog-title]) {
    margin: 0;
    font-size: var(--fs-body);
    font-weight: 600;
  }
  header :global(.close) {
    position: absolute;
    top: 8px;
    right: 8px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 2px;
  }
  .tabs button {
    display: grid;
    justify-items: center;
    gap: 3px;
    min-width: 76px;
    padding: 6px 8px 5px;
    border: 0;
    border-radius: var(--r-control);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-cap);
    cursor: default;
    transition:
      background-color 140ms var(--ease),
      color 140ms var(--ease);
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button[aria-selected='true'] {
    background: var(--sel);
    color: var(--accent);
  }
  .tabs button[aria-selected='true'] span {
    color: var(--text);
  }
  .body {
    flex: 1;
    overflow: auto;
    padding: 20px 24px 28px;
  }
  .pane {
    animation: pane-in 220ms var(--ease);
  }
  @keyframes pane-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  /* System Settings rows: label on the left, control on the right. */
  .pane :global(section > .fld) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  /* A block that needs the full width (theme grid): label above, content below. */
  .pane :global(section > .fld.wide) {
    display: grid;
    justify-content: stretch;
    gap: 8px;
  }
  .pane :global(section > .fld > .lbl) {
    color: var(--text);
    font-size: var(--fs-body);
  }

  .pane > .note:last-child {
    text-align: center;
  }
  @media (max-width: 640px) {
    :global(.prefs) {
      inset: auto 0 0 0;
      width: 100vw;
      height: 92vh;
      transform: none;
      border-radius: var(--r-sheet) var(--r-sheet) 0 0;
      animation: none;
    }
  }
</style>
