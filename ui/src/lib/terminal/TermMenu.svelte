<script lang="ts">
  import { ContextMenu } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';

  /** The terminal's right-click menu. */
  let {
    hasHost,
    selected,
    oncopy,
    onpaste,
    onselectall,
    onfind,
    onclear,
    onanother,
    onfiles,
    onclosed,
    children
  }: {
    hasHost: boolean;
    selected: boolean;
    oncopy: () => void;
    onpaste: () => void;
    onselectall: () => void;
    onfind: () => void;
    onclear: () => void;
    onanother: () => void;
    /** Opens the shell's current folder in Files; absent when the folder is unknown. */
    onfiles?: () => void;
    onclosed: () => void;
    children: Snippet;
  } = $props();
</script>

{#snippet item(label: string, onSelect: () => void, disabled = false)}
  <ContextMenu.Item class="menu-item" {disabled} {onSelect}>{label}</ContextMenu.Item>
{/snippet}

<ContextMenu.Root>
  <ContextMenu.Trigger class="term-trigger">{@render children()}</ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content
      class="menu"
      onCloseAutoFocus={(e) => {
        e.preventDefault();
        onclosed();
      }}
    >
      {@render item(m.keys_copy(), oncopy, !selected)}
      {@render item(m.keys_paste(), onpaste)}
      {@render item(m.keys_select_all(), onselectall)}
      <div class="menu-sep"></div>
      {@render item(m.keys_find(), onfind)}
      {@render item(m.term_menu_clear(), onclear)}
      {#if hasHost}
        <div class="menu-sep"></div>
        {@render item(m.term_menu_open_files(), () => onfiles?.(), !onfiles)}
        {@render item(m.menu_connect_another(), onanother)}
      {/if}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
