<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { Folder as FolderIcon, Search, Terminal, Zap } from '@lucide/svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { connectAdhoc, connectHost } from '$lib/stores/actions';
  import { recent } from '$lib/stores/recent.svelte';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { snippets as library } from '$lib/snippets/snippets.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { vault } from '$lib/stores/vault.svelte';
  import { rank } from '$lib/util/fuzzy';
  import { parseQuickConnect } from '$lib/util/quickconnect';
  import { fkey, folderChain, inheritedFlag } from '$lib/util/tree-model';

  interface Item {
    id: string;
    group: 'quick' | 'host' | 'folder' | 'snippet' | 'action';
    label: string;
    hint?: string;
    flag?: string | null;
    icon?: 'terminal' | 'folder' | 'snippet' | 'action';
    run: (shift: boolean) => void;
  }

  let query = $state('');
  let active = $state(0);
  let listEl = $state<HTMLDivElement>();

  $effect(() => {
    if (ui.paletteOpen) {
      query = '';
      active = 0;
    }
  });

  const close = () => (ui.paletteOpen = false);

  function reveal(id: string) {
    for (const f of folderChain(tree.data, id)) void tree.setExpanded(f.id, true);
    ui.sidebarOpen = true;
    tree.select([fkey(id)], fkey(id));
  }

  const actions = $derived<Item[]>([
    {
      id: 'a:host',
      group: 'action',
      label: m.action_new_host(),
      run: () => ui.openHostEditor(null)
    },
    {
      id: 'a:folder',
      group: 'action',
      label: m.action_new_folder(),
      run: () => ui.openFolderEditor(null)
    },
    ...(vault.canLock
      ? [
          {
            id: 'a:lock',
            group: 'action' as const,
            label: m.action_lock(),
            run: () => void vault.lock()
          }
        ]
      : []),
    {
      id: 'a:settings',
      group: 'action',
      label: m.settings_title(),
      run: () => (ui.settingsOpen = true)
    },
    {
      id: 'a:keys',
      group: 'action',
      label: m.keys_title(),
      run: () => (ui.hotkeysOpen = true)
    },
    {
      id: 'a:theme',
      group: 'action',
      label: m.action_toggle_theme(),
      run: () => settings.toggleTheme()
    }
  ]);

  const all = $derived<Item[]>([
    ...tree.data.hosts.map<Item>((h) => ({
      id: 'h:' + h.id,
      group: 'host',
      label: h.label,
      hint: `${h.username ? h.username + '@' : ''}${h.hostname}`,
      flag: h.country_code ?? inheritedFlag(tree.data, h.folder_id)?.value ?? null,
      icon: 'terminal',
      run: (shift) => connectHost(h.id, shift ? 'sftp' : 'terminal')
    })),
    ...tree.data.folders.map<Item>((f) => ({
      id: 'f:' + f.id,
      group: 'folder',
      label: f.name,
      flag: f.country_code,
      icon: 'folder',
      run: () => reveal(f.id)
    })),
    ...library.items.map<Item>((s) => ({
      id: 's:' + s.id,
      group: 'snippet',
      label: s.name,
      hint: s.body.split('\n')[0],
      icon: 'snippet',
      // Shift = choose servers ("Run on…").
      run: (shift) => {
        if (shift) library.run(s, 'many');
        else if (sessions.active) library.run(s);
        else ui.toast(m.palette_no_terminal());
      }
    })),
    ...actions
  ]);

  const items = $derived.by<Item[]>(() => {
    const q = query.trim();
    const quick = parseQuickConnect(q);
    const head: Item[] = quick
      ? [
          {
            id: 'q',
            group: 'quick',
            label: m.palette_connect_to({ target: q }),
            icon: 'terminal',
            run: () => connectAdhoc(quick)
          }
        ]
      : [];
    if (!q) {
      // Recently opened servers first, then the rest of the catalog.
      const hosts = all.filter((i) => i.group === 'host');
      const order = new Map(recent.items.map((e, i) => [`h:${e.id}`, i]));
      const sorted = [...hosts].sort((a, b) => (order.get(a.id) ?? 99) - (order.get(b.id) ?? 99));
      return [...sorted.slice(0, 8), ...actions];
    }
    const hits = rank(q, all, (i) => [i.label, i.hint ?? '']).slice(0, 40);
    return [...head, ...hits];
  });

  const groupLabel = (g: Item['group']) =>
    ({
      quick: '',
      host: m.palette_hosts(),
      folder: m.palette_folders(),
      snippet: m.palette_snippets(),
      action: m.palette_actions()
    })[g];

  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') active = Math.min(items.length - 1, active + 1);
    else if (e.key === 'ArrowUp') active = Math.max(0, active - 1);
    else if (e.key === 'Enter') {
      const it = items[active];
      if (it) {
        close();
        it.run(e.shiftKey);
      }
    } else return;
    e.preventDefault();
    queueMicrotask(() =>
      listEl?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
    );
  }
  const focus = (n: HTMLElement) => n.focus();
</script>

<Dialog.Root bind:open={ui.paletteOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content
      class="palette"
      aria-label={m.palette_title()}
      onOpenAutoFocus={(e) => e.preventDefault()}
    >
      <Dialog.Title class="sr">{m.palette_title()}</Dialog.Title>
      <div class="field">
        <Search size={16} />
        <input
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="pal-list"
          aria-activedescendant={'pal-' + active}
          placeholder={m.palette_placeholder()}
          bind:value={query}
          oninput={() => (active = 0)}
          onkeydown={key}
          autocomplete="off"
          spellcheck="false"
          use:focus
        />
      </div>
      <div class="list" id="pal-list" role="listbox" bind:this={listEl}>
        {#each items as it, i (it.id)}
          {#if groupLabel(it.group) && items[i - 1]?.group !== it.group}
            <div class="grp">{groupLabel(it.group)}</div>
          {/if}
          <div
            id={'pal-' + i}
            class="menu-item"
            role="option"
            aria-selected={i === active}
            tabindex="-1"
            data-active={i === active}
            data-highlighted={i === active ? '' : undefined}
            onclick={() => (close(), it.run(false))}
            onkeydown={null}
            onpointermove={() => (active = i)}
          >
            <span class="ic">
              {#if it.group === 'host' || it.group === 'folder'}
                {#if it.flag}<Flag
                    code={it.flag}
                    size={16}
                  />{:else if it.icon === 'folder'}<FolderIcon size={16} />{:else}<Flag
                    code={null}
                    size={16}
                  />{/if}
              {:else if it.icon === 'terminal'}<Terminal size={16} />
              {:else}<Zap size={16} />{/if}
            </span>
            <span class="label">{it.label}</span>
            {#if it.hint}<span class="hint">{it.hint}</span>{/if}
          </div>
        {/each}
        {#if items.length === 0}<div class="none">{m.palette_empty()}</div>{/if}
      </div>
      <footer>
        <span><span class="kbd">↵</span> {m.palette_run()}</span><span
          ><span class="kbd">⇧↵</span> SFTP</span
        >
      </footer>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.palette) {
    position: fixed;
    z-index: 51;
    top: min(18vh, 140px);
    left: 50%;
    width: min(560px, calc(100vw - 24px));
    transform: translateX(-50%);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: var(--shadow);
    overflow: hidden;
    animation: pop 120ms var(--ease);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(-4px);
    }
  }
  :global(.sr) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    height: 48px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  .field input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 15px;
    outline: none;
  }
  .list {
    max-height: min(360px, 50vh);
    overflow: auto;
    padding: 4px;
  }
  .grp {
    padding: 8px 8px 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .menu-item {
    height: 32px;
    cursor: pointer;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 16px;
    color: var(--muted);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
    min-width: 0;
  }
  .none {
    padding: 24px;
    text-align: center;
    color: var(--muted);
  }
  footer {
    display: flex;
    gap: 16px;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
  }
</style>
