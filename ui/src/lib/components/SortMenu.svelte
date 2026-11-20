<script lang="ts">
  import { ArrowDownUp, Check } from '@lucide/svelte';
  import { DropdownMenu } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { HOST_SORTS, type HostSort } from '$lib/util/host-sort';

  /**
   * Server order picker. `compact` renders an icon button (sidebar), otherwise a pill with the
   * current order's name (overview). A non-manual order is highlighted so it is never forgotten.
   */
  let {
    value,
    onchange,
    compact = false,
    manualLabel = m.sort_manual()
  }: {
    value: HostSort;
    onchange: (sort: HostSort) => void;
    compact?: boolean;
    manualLabel?: string;
  } = $props();

  const labels: Record<HostSort, () => string> = {
    manual: () => manualLabel,
    name: m.sort_name,
    last: m.sort_last,
    country: m.sort_country
  };
  const title = $derived(`${m.sort_title()}: ${labels[value]()}`);
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      {#if compact}
        <button
          {...props}
          class="icon-btn sort-btn"
          class:active={value !== 'manual'}
          aria-label={title}
          {title}><ArrowDownUp size={14} /></button
        >
      {:else}
        <button {...props} class="sort-pill" class:active={value !== 'manual'} {title}
          ><ArrowDownUp size={13} /><span>{labels[value]()}</span></button
        >
      {/if}
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content class="menu" align="end" sideOffset={4}>
      <DropdownMenu.Group>
        <DropdownMenu.GroupHeading class="menu-heading">{m.sort_title()}</DropdownMenu.GroupHeading>
        {#each HOST_SORTS as s (s)}
          <DropdownMenu.Item class="menu-item" onSelect={() => onchange(s)}>
            <span class="tick"
              >{#if s === value}<Check size={14} />{/if}</span
            >{labels[s]()}
          </DropdownMenu.Item>
        {/each}
      </DropdownMenu.Group>
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  .tick {
    display: inline-grid;
    place-items: center;
    width: 14px;
  }
  :global(.menu-heading) {
    padding: 4px 10px 2px;
    color: var(--muted);
    font-size: var(--fs-cap);
    font-weight: 600;
  }
  .sort-btn.active {
    opacity: 1 !important;
    color: var(--accent);
  }
  .sort-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
  }
  .sort-pill:hover,
  .sort-pill[data-state='open'] {
    background: var(--hover);
    color: var(--text);
  }
  .sort-pill.active {
    color: var(--text);
  }
  .sort-pill.active :global(svg) {
    color: var(--accent);
  }
</style>
