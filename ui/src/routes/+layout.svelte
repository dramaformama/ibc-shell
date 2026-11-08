<script lang="ts">
  import '../app.css';
  import '$lib/forms.css';
  import { onMount } from 'svelte';
  import Shell from '$lib/components/Shell.svelte';
  import VaultScreen from '$lib/components/VaultScreen.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { start } from '$lib/start/start.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { recent } from '$lib/stores/recent.svelte';
  import { lastSeen } from '$lib/stores/lastseen.svelte';
  import { geo } from '$lib/map/geoip.svelte';
  import { connectHost, connectInBackground } from '$lib/stores/actions';
  import { buildMenu, onTrayOpen, pushTray, type TrayEntry } from '$lib/tray/tray.svelte';
  import { snippets } from '$lib/snippets/snippets.svelte';
  import { stats } from '$lib/stores/stats.svelte';
  import { services } from '$lib/services/services.svelte';
  import { dockerRes } from '$lib/services/dockerres.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { watchIdle } from '$lib/util/autolock';
  import * as m from '$lib/paraglide/messages.js';
  import { setUnitLabels } from '$lib/stats/format';

  let { children } = $props();
  setUnitLabels([m.unit_b(), m.unit_kb(), m.unit_mb(), m.unit_gb(), m.unit_tb()], m.unit_per_s());

  onMount(() => {
    void vault.refresh();
    const off = onTrayOpen((id) => {
      if (vault.state === 'Unlocked') connectHost(id, 'status');
    });
    return () => void off.then((f) => f());
  });

  // Menu-bar icon: keep open servers sampled and send their lines (ADR-013).
  $effect(() => {
    if (vault.state !== 'Unlocked' || !settings.tray) return;
    const stops = sessions.groups
      .map((g) => sessions.groupSession(g))
      .filter((sid): sid is string => !!sid)
      .map((sid) => stats.watch(sid));
    return () => stops.forEach((stop) => stop());
  });
  $effect(() => {
    if (vault.state === 'Loading') return;
    if (!settings.tray) return pushTray(null);
    if (vault.state !== 'Unlocked') return pushTray(buildMenu([], true));
    const entries: TrayEntry[] = [];
    const open = new Set<string>();
    for (const g of sessions.groups) {
      const hostId = g.current.hostId;
      if (!hostId) continue;
      open.add(hostId);
      const s = stats.get(sessions.groupSession(g));
      const phase = sessions.groupPhase(g);
      entries.push({ hostId, label: g.label, phase, d: s && !s.stale ? s.latest : null });
    }
    for (const id of lastSeen.watched) {
      const h = tree.host(id);
      if (h && !open.has(id)) entries.push({ hostId: id, label: h.label, phase: null, d: null });
    }
    pushTray(buildMenu(entries, false));
  });

  // Load data when the vault opens; wipe it when it locks.
  $effect(() => {
    if (vault.state === 'Unlocked') {
      void settings.load().then(() => settings.checkUpdate());
      void Promise.all([tree.load(), lastSeen.load()]).then(() => {
        for (const id of lastSeen.watched) if (tree.host(id)) connectInBackground(id);
      });
      void recent.load();
      void start.load();
      void geo.load();
      void snippets.load();
    } else if (vault.state === 'Locked') {
      tree.data = { folders: [], hosts: [] };
      tree.loaded = false;
      sessions.clear();
      start.clear();
      stats.clear();
      services.clear();
      dockerRes.clear();
      lastSeen.clear();
      geo.clear();
      snippets.clear();
    }
  });

  // No master password (ADR-014): create / open the keychain vault by itself.
  $effect(() => {
    if (
      vault.state === 'Uninitialized' ||
      (vault.state === 'Locked' && vault.mode === 'OsKeychain')
    ) {
      if (!vault.error) void vault.auto();
    }
  });

  // Idle auto-lock of a not-yet-migrated password vault.
  $effect(() => {
    if (vault.state !== 'Unlocked' || !vault.canLock) return;
    return watchIdle(settings.autolockMinutes, () => void vault.lock());
  });
</script>

{#if vault.state === 'Unlocked'}
  <Shell>{@render children()}</Shell>
{:else if vault.state !== 'Loading'}
  <VaultScreen />
{/if}
