<script lang="ts">
  import { Popover } from 'bits-ui';
  import { ChevronDown } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import { flagName, searchFlags, type FlagLocale } from '$lib/flags/flags';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import * as m from '$lib/paraglide/messages.js';
  import { settings } from '$lib/stores/settings.svelte';

  interface Props {
    /** Own flag (null = none/inherit). */
    value: string | null;
    /** Inherited flag shown at 70% opacity when `value` is null. */
    inherited?: { value: string; from: string | null } | null;
    onchange: (code: string | null) => void;
  }
  let { value, inherited = null, onchange }: Props = $props();

  let open = $state(false);
  let query = $state('');
  let active = $state(0);
  let list = $state<HTMLDivElement>();
  const locale = getLocale() as FlagLocale;

  const recent = $derived(settings.recentFlags);
  const found = $derived(searchFlags(query, recent));
  /** Flat list: null = "None", then (optionally) recent, then all. */
  const items = $derived.by(() => {
    const out: { code: string | null; section: string | null }[] = [{ code: null, section: null }];
    const recentHits = query ? [] : recent.filter((c) => found.includes(c));
    for (const c of recentHits) out.push({ code: c, section: 'recent' });
    for (const c of found) if (!recentHits.includes(c)) out.push({ code: c, section: 'all' });
    return out;
  });

  const shown = $derived(value ?? inherited?.value ?? null);

  function choose(code: string | null) {
    if (code) settings.pushRecentFlag(code);
    onchange(code);
    open = false;
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') active = Math.min(items.length - 1, active + 1);
    else if (e.key === 'ArrowUp') active = Math.max(0, active - 1);
    else if (e.key === 'Enter') {
      const it = items[active];
      if (it) choose(it.code);
    } else return;
    e.preventDefault();
    queueMicrotask(() =>
      list?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
    );
  }
  const focusOnMount = (n: HTMLInputElement) => n.focus();
</script>

<Popover.Root bind:open onOpenChange={(o) => o && ((query = ''), (active = 0))}>
  <Popover.Trigger class="input fp-trigger" aria-label={m.field_flag()}>
    <span class="fp-val" class:inh={!value && inherited}>
      <Flag code={shown} size={20} />
      <span class="fp-name">
        {#if value}{flagName(value, locale)}{:else if inherited}{m.inherited_from({
            name: inherited.from ?? '',
            value: flagName(inherited.value, locale)
          })}{:else}{m.flag_none()}{/if}
      </span>
    </span>
    <ChevronDown size={14} />
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="menu fp-pop" sideOffset={4} align="start">
      <input
        class="input"
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls="fp-list"
        aria-activedescendant={'fp-' + active}
        placeholder={m.flag_search()}
        bind:value={query}
        oninput={() => (active = 0)}
        onkeydown={key}
        use:focusOnMount
      />
      <div class="fp-list" id="fp-list" role="listbox" bind:this={list}>
        {#each items as it, i (it.section + ':' + it.code)}
          {#if it.section && items[i - 1]?.section !== it.section}
            <div class="fp-sec">{it.section === 'recent' ? m.flag_recent() : m.flag_all()}</div>
          {/if}
          <div
            id={'fp-' + i}
            class="menu-item"
            role="option"
            aria-selected={(value ?? null) === it.code}
            tabindex="-1"
            data-active={i === active}
            data-highlighted={i === active ? '' : undefined}
            onclick={() => choose(it.code)}
            onkeydown={null}
            onpointermove={() => (active = i)}
          >
            {#if it.code}
              <Flag code={it.code} size={16} />
              <span class="fp-label">{flagName(it.code, locale)}</span>
              <span class="fp-code">{it.code.toUpperCase()}</span>
            {:else}
              <span class="fp-label">{m.flag_none()}</span>
            {/if}
          </div>
        {/each}
        {#if items.length === 1 && query}<div class="fp-sec">{m.flag_no_results()}</div>{/if}
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.fp-trigger) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    cursor: pointer;
    text-align: left;
    height: var(--control-h);
  }
  .fp-val {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .fp-val.inh :global(.flag) {
    opacity: 0.7;
  }
  .fp-val.inh .fp-name {
    color: var(--muted);
  }
  .fp-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.fp-pop) {
    width: 280px;
    display: grid;
    gap: 6px;
    padding: 6px;
  }
  .fp-list {
    max-height: 280px;
    overflow: auto;
  }
  .fp-sec {
    padding: 8px 8px 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .fp-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fp-code {
    color: var(--muted);
    font-size: 12px;
  }
</style>
