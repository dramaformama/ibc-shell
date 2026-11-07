<script lang="ts">
  import { Activity, X, Zap } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import SnippetsPanel from '$lib/snippets/SnippetsPanel.svelte';
  import type { TabGroup } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import ServicesPane from '$lib/services/ServicesPane.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import MetricsPane from './MetricsPane.svelte';

  /** Inspector column next to the shell: live metrics or the snippet library. */
  let { group }: { group: TabGroup } = $props();
  const sid = $derived(sessions.groupSession(group));
</script>

<aside class="panel" aria-label={m.stat_inspector()}>
  <div class="top">
    <div class="seg" role="tablist" aria-label={m.stat_inspector()}>
      <button
        role="tab"
        aria-selected={ui.panelTab === 'metrics'}
        onclick={() => (ui.panelTab = 'metrics')}><Activity size={14} />{m.panel_metrics()}</button
      >
      <button
        role="tab"
        aria-selected={ui.panelTab === 'snippets'}
        onclick={() => (ui.panelTab = 'snippets')}><Zap size={14} />{m.panel_snippets()}</button
      >
    </div>
    <button
      class="icon-btn"
      onclick={() => ui.toggleInspector()}
      aria-label={m.action_hide()}
      title={m.action_hide()}><X size={15} /></button
    >
  </div>
  <div class="scroll">
    {#if ui.panelTab === 'metrics'}
      <MetricsPane {group} />
      {#if sid}<ServicesPane {sid} compact />{/if}
    {:else}
      <SnippetsPanel />
    {/if}
  </div>
</aside>

<style>
  .panel {
    width: 340px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--bg);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 12px 4px 14px;
  }
  .top .seg {
    flex: 1;
  }
  .top .seg button {
    flex: 1;
    justify-content: center;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 14px 16px;
  }
  @media (max-width: 1000px) {
    .panel {
      position: absolute;
      z-index: 8;
      inset: 0 0 0 auto;
      width: min(340px, 86%);
      box-shadow: var(--shadow);
    }
  }
</style>
