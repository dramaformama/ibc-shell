<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import ServerCard from './ServerCard.svelte';

  const groups = $derived(sessions.groups);
  const online = $derived(groups.filter((g) => sessions.groupPhase(g) === 'connected').length);
</script>

<section class="dashboard" aria-label={m.dash_title()}>
  <header>
    <h1>{m.dash_title()}</h1>
    <div class="totals tnum">
      <span>{m.dash_total()} <b>{groups.length}</b></span>
      <span><i class="dot" data-phase="connected"></i>{m.dash_online()} <b>{online}</b></span>
      <span>{m.dash_offline()} <b>{groups.length - online}</b></span>
    </div>
  </header>
  <div class="servers">
    {#each groups as group (group.key)}<ServerCard {group} />{/each}
  </div>
</section>

<style>
  .dashboard {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    padding: clamp(20px, 3vw, 40px);
  }
  header {
    margin-bottom: 24px;
  }
  h1 {
    margin: 0 0 16px;
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .totals {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .totals span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
  }
  b {
    color: var(--text);
  }
  .servers {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(340px, 100%), 1fr));
    gap: 18px;
    align-items: start;
  }
</style>
