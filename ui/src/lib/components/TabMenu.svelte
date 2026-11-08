<script lang="ts">
  import { ContextMenu } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { closeServer, connectAnother, switchKind } from '$lib/stores/actions';
  import { sessions, type TabGroup, type TabKind } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';

  let { group, children }: { group: TabGroup; children: Snippet<[Record<string, unknown>]> } =
    $props();

  const hostId = $derived(group.current.hostId);

  function show(kind: TabKind) {
    sessions.select(group.current.id);
    switchKind(kind);
  }
</script>

{#snippet item(label: string, onSelect: () => void, danger = false)}
  <ContextMenu.Item class="menu-item {danger ? 'danger' : ''}" {onSelect}>{label}</ContextMenu.Item>
{/snippet}

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}{@render children(props)}{/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="menu">
      {#if hostId}
        {@const id = hostId}
        {@render item(m.tab_status(), () => show('status'))}
        {@render item(m.tab_terminal(), () => show('terminal'))}
        {@render item(m.tab_files(), () => show('sftp'))}
        <div class="menu-sep"></div>
        {@render item(m.menu_connect_another(), () => connectAnother(id))}
        {@render item(m.menu_edit(), () => ui.openHostEditor(id))}
        <div class="menu-sep"></div>
      {/if}
      {@render item(m.tab_disconnect(), () => closeServer(group.key), true)}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
