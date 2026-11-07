<script lang="ts">
  import { sessions } from '$lib/stores/sessions.svelte';
  import { drag, sftpLayout } from './layout.svelte';
  import { Channel } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { api, type Endpoint, type SessionEvent, type SftpId } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import type { Tab } from '$lib/stores/sessions.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';
  import ConflictDialog from './ConflictDialog.svelte';
  import DeleteDialog from './DeleteDialog.svelte';
  import NameDialog from './NameDialog.svelte';
  import { Pane as PaneModel } from './paneState.svelte';
  import PaneView from './Pane.svelte';
  import PermsDialog from './PermsDialog.svelte';
  import TransferDrawer from './TransferDrawer.svelte';
  import { transfers } from './transfers.svelte';
  import type { Actions, Source } from './model';
  import type { Entry } from '$lib/api';
  import './sftp.css';

  let { tab }: { tab: Tab } = $props();

  type Side = 'left' | 'right';
  const panes: Record<Side, PaneModel> = { left: new PaneModel(), right: new PaneModel() };
  let sftpId = $state<SftpId | null>(null);
  let status = $state<'connecting' | 'ready' | 'failed'>('connecting');
  let failure = $state('');
  let active = $state<Side>('left');
  let sel = $state<Record<Side, string>>({ left: 'local', right: 'remote' });
  let osOver = $state<Side | null>(null);

  const sources = $derived<Source[]>([
    { value: 'local', label: m.sftp_local(), ep: { kind: 'local' } },
    ...(sftpId
      ? [{ value: 'remote', label: tab.label, ep: { kind: 'remote', sftp_id: sftpId } as Endpoint }]
      : [])
  ]);
  const other = (s: Side): Side => (s === 'left' ? 'right' : 'left');

  async function attach(side: Side, value: string) {
    const src = sources.find((s) => s.value === value);
    if (!src) return;
    sel[side] = value;
    panes[side].filter = '';
    await panes[side].attach(src.ep);
    if (panes[side].error) ui.toast(errorText(panes[side].error!), 'error');
  }

  onMount(() => {
    let closed = false;
    let id: SftpId | null = null;
    void transfers.start();
    void (async () => {
      if (!tab.hostId) {
        status = 'failed';
        failure = m.sftp_no_host();
        return;
      }
      const ch = new Channel<SessionEvent>();
      ch.onmessage = (e) => {
        if (e.type === 'Failed') {
          status = 'failed';
          failure = errorText(e.error);
        } else if (e.type === 'Disconnected') {
          status = 'failed';
          failure = e.reason;
        }
      };
      const group = sessions.groups.find((g) => g.tabs.some((t) => t.id === tab.id));
      const sid = group ? sessions.groupSession(group) : null;
      const r = await api.sftpOpen(
        sid ? { kind: 'session', session_id: sid } : { kind: 'host', host_id: tab.hostId },
        ch
      );
      if (r.status === 'error') {
        status = 'failed';
        failure = errorText(r.error);
        return;
      }
      id = r.data;
      if (closed) {
        void api.sftpClose(id);
        return;
      }
      sftpId = id;
      status = 'ready';
      await Promise.all([attach('left', 'local'), attach('right', 'remote')]);
    })();
    const offOs = listenOsDrop();
    return () => {
      closed = true;
      if (id) void api.sftpClose(id);
      sessions.setPhase(tab.id, 'disconnected');
      void offOs.then((f) => f());
    };
  });

  // Refresh both listings whenever a transfer finishes.
  let seenTick = 0;
  $effect(() => {
    const t = transfers.doneTick;
    if (t !== seenTick) {
      seenTick = t;
      void panes.left.refresh();
      void panes.right.refresh();
    }
  });

  /** OS → pane drops (Tauri gives local paths and a physical-pixel position). */
  async function listenOsDrop(): Promise<() => void> {
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      return await getCurrentWebview().onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === 'leave') {
          osOver = null;
          return;
        }
        if (p.type === 'enter' || p.type === 'over' || p.type === 'drop') {
          const dpr = window.devicePixelRatio || 1;
          const el = document.elementFromPoint(p.position.x / dpr, p.position.y / dpr);
          const side = el?.closest<HTMLElement>('[data-side]')?.dataset.side as Side | undefined;
          const target = side && panes[side].endpoint.kind === 'remote' ? side : null;
          osOver = p.type === 'drop' ? null : target;
          if (p.type === 'drop' && target && p.paths.length) {
            void run(
              transfers.enqueue(
                { kind: 'local' },
                panes[target].endpoint,
                p.paths,
                panes[target].path
              )
            );
          }
        }
      });
    } catch {
      return () => {};
    }
  }

  async function run(p: Promise<boolean>) {
    if (!(await p)) ui.toast(m.sftp_enqueue_failed(), 'error');
  }

  function send(from: Side, names: string[], dstDir: string) {
    const src = panes[from];
    const dst = panes[other(from)];
    void run(transfers.enqueue(src.endpoint, dst.endpoint, names.map(src.pathOf), dstDir));
  }

  // ---- dialogs ----
  let nameFor = $state<{ side: Side; entry: Entry | null } | null>(null);
  let nameOpen = $state(false);
  let deleting = $state<{ side: Side; names: string[]; folders: number } | null>(null);
  let permsFor = $state<{ side: Side; name: string; mode: number } | null>(null);

  async function report(e: Promise<import('$lib/api').ApiError | null>) {
    const err = await e;
    if (err) ui.toast(errorText(err), 'error');
  }

  function actions(side: Side): Actions {
    const pane = panes[side];
    const to = panes[other(side)];
    const here = pane.endpoint.kind;
    const there = to.endpoint.kind;
    return {
      open: (e) => (e.kind === 'dir' ? void pane.open(e) : send(side, [e.name], to.path)),
      rename: (e) => {
        nameFor = { side, entry: e };
        nameOpen = true;
      },
      remove: () => {
        deleting = {
          side,
          names: pane.selected.map((e) => e.name),
          folders: pane.selected.filter((e) => e.kind === 'dir').length
        };
      },
      perms: (e) => (permsFor = { side, name: e.name, mode: e.mode }),
      copyPath: (e) => void navigator.clipboard?.writeText(pane.pathOf(e.name)),
      newFolder: () => {
        nameFor = { side, entry: null };
        nameOpen = true;
      },
      transfer: () =>
        send(
          side,
          pane.selected.map((e) => e.name),
          to.path
        ),
      transferLabel:
        here === 'local' && there === 'remote'
          ? m.sftp_upload()
          : here === 'remote' && there === 'local'
            ? m.sftp_download()
            : m.sftp_copy_other(),
      canPerms: here === 'remote',
      canTransfer: status === 'ready'
    };
  }
  const acts = $derived({ left: actions('left'), right: actions('right') });

  $effect(() =>
    sessions.setPhase(
      tab.id,
      status === 'ready' ? 'connected' : status === 'failed' ? 'failed' : 'connecting'
    )
  );

  let panesEl = $state<HTMLDivElement | null>(null);
  function startSplit(e: PointerEvent) {
    const w = panesEl?.clientWidth ?? 1;
    const s0 = sftpLayout.split;
    drag(
      e,
      (dx) => sftpLayout.setSplit(s0 + dx / w),
      () => sftpLayout.save()
    );
  }
  function splitKey(e: KeyboardEvent) {
    const d = e.key === 'ArrowLeft' ? -0.05 : e.key === 'ArrowRight' ? 0.05 : 0;
    if (!d) return;
    sftpLayout.setSplit(sftpLayout.split + d);
    sftpLayout.save();
  }
</script>

<div
  class="sf-screen"
  style:--sf-c-size="{sftpLayout.cols.size}px"
  style:--sf-c-mtime="{sftpLayout.cols.mtime}px"
  style:--sf-c-mode="{sftpLayout.cols.mode}px"
>
  {#if status === 'failed'}
    <div class="sf-state" role="alert">
      <p>{m.sftp_connect_failed()}</p>
      <p class="muted">{failure}</p>
    </div>
  {:else if status === 'connecting'}
    <div class="sf-state"><p class="muted">{m.sftp_connecting()}</p></div>
  {:else}
    <div
      class="sf-panes"
      bind:this={panesEl}
      style:grid-template-columns="minmax(0, {sftpLayout.split}fr) minmax(0, {1 -
        sftpLayout.split}fr)"
    >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="sf-split"
        role="separator"
        aria-orientation="vertical"
        aria-label={m.sftp_split()}
        aria-valuenow={Math.round(sftpLayout.split * 100)}
        aria-valuemin="20"
        aria-valuemax="80"
        tabindex="0"
        style:left="{sftpLayout.split * 100}%"
        onpointerdown={startSplit}
        onkeydown={splitKey}
        ondblclick={() => (sftpLayout.setSplit(0.5), sftpLayout.save())}
      ></div>
      {#each ['left', 'right'] as const as side (side)}
        <div class="sf-slot" data-side={side}>
          <PaneView
            pane={panes[side]}
            id={side}
            {sources}
            sourceValue={sel[side]}
            active={active === side}
            hover={osOver === side}
            act={acts[side]}
            onSource={(v) => void attach(side, v)}
            onDrop={send}
            onFocus={() => (active = side)}
          />
        </div>
      {/each}
    </div>
  {/if}
  <TransferDrawer />
</div>

<NameDialog
  bind:open={nameOpen}
  title={nameFor?.entry ? m.menu_rename() : m.action_new_folder()}
  initial={nameFor?.entry?.name ?? ''}
  confirm={nameFor?.entry ? m.menu_rename() : m.action_create()}
  submit={(name) => {
    const f = nameFor;
    if (!f) return;
    void report(f.entry ? panes[f.side].rename(f.entry.name, name) : panes[f.side].mkdir(name));
  }}
/>
<DeleteDialog
  names={deleting?.names ?? null}
  folders={deleting?.folders ?? 0}
  cancel={() => (deleting = null)}
  confirm={() => {
    const d = deleting;
    deleting = null;
    if (d) void report(panes[d.side].remove(d.names));
  }}
/>
<PermsDialog
  target={permsFor}
  cancel={() => (permsFor = null)}
  apply={(mode) => {
    const p = permsFor;
    permsFor = null;
    if (p) void report(panes[p.side].chmod(p.name, mode));
  }}
/>
<ConflictDialog />
