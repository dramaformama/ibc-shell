<script lang="ts">
  import { Plus } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { mod } from '$lib/util/platform';

  const tab = $derived(sessions.active);
</script>

<!-- Terminal / SFTP content is mounted here by terminal-ui and sftp-ui for the active tab. -->
<div class="stage" data-tab-id={tab?.id} data-tab-kind={tab?.kind}>
  {#if tab}
    <div class="host-card">
      <Flag code={tab.flag} size={20} />
      <span>{tab.label}</span>
    </div>
  {:else}
    <div class="welcome">
      <h1>{m.app_name()}</h1>
      <p>{m.welcome_hint()}</p>
      <div class="actions">
        <button class="btn btn-primary" onclick={() => ui.openHostEditor(null)}
          ><Plus size={14} />{m.action_new_host()}</button
        >
        <button class="btn" onclick={() => (ui.paletteOpen = true)}
          >{m.palette_open()} <span class="kbd">{mod('K')}</span></button
        >
      </div>
    </div>
  {/if}
</div>

<style>
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }
  .welcome {
    display: grid;
    gap: 8px;
    justify-items: center;
    text-align: center;
    padding: 24px;
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  p {
    margin: 0 0 12px;
    color: var(--muted);
    max-width: 36ch;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }
  .host-card {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
</style>
