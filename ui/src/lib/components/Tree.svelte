<script lang="ts">
  import { Server } from '@lucide/svelte';
  import EmptyState from './EmptyState.svelte';
  import { ContextMenu } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { connectHost } from '$lib/stores/actions';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { resolveDrop, siblings, type Drop, type NodeKey, type Row } from '$lib/util/tree-model';
  import TreeMenu from './TreeMenu.svelte';
  import TreeRowContent from './TreeRowContent.svelte';
  import './tree.css';

  const ROW = 30;
  const OVERSCAN = 8;
  const INDENT = 14;

  let scroller = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let viewH = $state(400);
  let anchor: NodeKey | null = null;
  let ctx = $state.raw<{ row: Row | null; keys: NodeKey[] }>({ row: null, keys: [] });

  const rows = $derived(tree.rows);
  const start = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const end = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW) + OVERSCAN));
  const visible = $derived(rows.slice(start, end));
  const indexOfKey = (k: NodeKey | null) => (k ? rows.findIndex((r) => r.key === k) : -1);

  // ── selection ────────────────────────────────────────────
  function onRowClick(e: MouseEvent, row: Row) {
    if (suppressClick) return (suppressClick = false);
    scroller?.focus({ preventScroll: true });
    if (e.shiftKey && anchor) {
      const a = indexOfKey(anchor);
      const b = indexOfKey(row.key);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      tree.select(
        rows.slice(lo, hi + 1).map((r) => r.key),
        row.key
      );
      return;
    }
    anchor = row.key;
    if (e.metaKey || e.ctrlKey) {
      const next = new Set(tree.selected);
      if (!next.delete(row.key)) next.add(row.key);
      tree.select(next, row.key);
      return;
    }
    tree.select([row.key], row.key);
    if (row.kind === 'folder' && !(e.target as HTMLElement).closest('.chev'))
      void tree.setExpanded(row.id, !row.expanded);
  }

  function moveCursor(i: number, extend: boolean) {
    const row = rows[Math.max(0, Math.min(rows.length - 1, i))];
    if (!row) return;
    if (extend && anchor) {
      const a = indexOfKey(anchor);
      const b = indexOfKey(row.key);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      tree.select(
        rows.slice(lo, hi + 1).map((r) => r.key),
        row.key
      );
    } else {
      anchor = row.key;
      tree.select([row.key], row.key);
    }
  }

  // Keep the cursor row visible (keyboard moves and "reveal folder" from the palette).
  $effect(() => {
    const i = indexOfKey(tree.cursor);
    if (i < 0 || !scroller) return;
    const top = i * ROW;
    if (top < scroller.scrollTop) scroller.scrollTop = top;
    else if (top + ROW > scroller.scrollTop + scroller.clientHeight)
      scroller.scrollTop = top + ROW - scroller.clientHeight;
  });

  function onKeyDown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === 'INPUT') return;
    const cur = indexOfKey(tree.cursor);
    const row = rows[cur];
    const mod = e.metaKey || e.ctrlKey;
    switch (e.key) {
      case 'ArrowDown':
        moveCursor(cur + 1, e.shiftKey);
        break;
      case 'ArrowUp':
        moveCursor(cur < 0 ? 0 : cur - 1, e.shiftKey);
        break;
      case 'Home':
        moveCursor(0, e.shiftKey);
        break;
      case 'End':
        moveCursor(rows.length - 1, e.shiftKey);
        break;
      case 'ArrowRight':
        if (row?.kind === 'folder') {
          if (!row.expanded) void tree.setExpanded(row.id, true);
          else moveCursor(cur + 1, false);
        }
        break;
      case 'ArrowLeft':
        if (row?.kind === 'folder' && row.expanded) void tree.setExpanded(row.id, false);
        else if (row?.parentId) moveCursor(indexOfKey('f:' + row.parentId), false);
        break;
      case 'Enter':
        if (!row) break;
        if (row.kind === 'host') connectHost(row.id, e.shiftKey ? 'sftp' : 'terminal');
        else void tree.setExpanded(row.id, !row.expanded);
        break;
      case ' ':
        if (row) {
          const next = new Set(tree.selected);
          if (!next.delete(row.key)) next.add(row.key);
          tree.select(next, row.key);
        }
        break;
      case 'F2':
        if (row) ui.renameKey = row.key;
        break;
      case 'Delete':
      case 'Backspace':
        if (tree.selected.size) ui.deleteKeys = [...tree.selected];
        break;
      case 'a':
        if (!mod) return;
        tree.select(
          rows.map((r) => r.key),
          tree.cursor ?? undefined
        );
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  // ── context menu ─────────────────────────────────────────
  function onContextMenu(e: MouseEvent, row: Row | null) {
    // The event must reach ContextMenu.Trigger, so the empty-area handler skips clicks on rows.
    if (!row && e.target instanceof Element && e.target.closest('[role="treeitem"]')) return;
    if (row && !tree.selected.has(row.key)) tree.select([row.key], row.key);
    ctx = {
      row,
      keys: row ? (tree.selected.has(row.key) ? ordered([...tree.selected]) : [row.key]) : []
    };
  }
  const ordered = (keys: NodeKey[]) => rows.filter((r) => keys.includes(r.key)).map((r) => r.key);

  // ── rename ───────────────────────────────────────────────
  async function commitRename(row: Row, value: string) {
    ui.renameKey = null;
    if (value.trim() && value.trim() !== row.label) await tree.rename(row.key, value);
    scroller?.focus({ preventScroll: true });
  }
  function cancelRename() {
    ui.renameKey = null;
    scroller?.focus({ preventScroll: true });
  }

  // ── drag and drop (pointer events) ───────────────────────
  interface Drag {
    keys: NodeKey[];
    x: number;
    y: number;
    drop: Drop | null;
    tail: boolean;
  }
  let drag = $state.raw<Drag | null>(null);
  let suppressClick = false;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let hoverKey: NodeKey | null = null;

  function onPointerDown(e: PointerEvent, row: Row) {
    if (e.button !== 0 || ui.renameKey || (e.target as HTMLElement).closest('.chev')) return;
    const sx = e.clientX;
    const sy = e.clientY;
    const move = (ev: PointerEvent) => {
      if (!drag && Math.hypot(ev.clientX - sx, ev.clientY - sy) < 5) return;
      if (!drag) {
        if (!tree.selected.has(row.key)) tree.select([row.key], row.key);
        drag = {
          keys: ordered([...tree.selected]),
          x: ev.clientX,
          y: ev.clientY,
          drop: null,
          tail: false
        };
      }
      track(ev);
    };
    const finish = (commit: boolean) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('keydown', esc, true);
      clearTimeout(hoverTimer);
      hoverKey = null;
      const d = drag;
      drag = null;
      if (d) {
        suppressClick = true;
        setTimeout(() => (suppressClick = false), 0);
        if (commit && d.drop) void tree.move(d.keys, d.drop);
      }
    };
    const up = () => finish(true);
    const esc = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') {
        ev.stopPropagation();
        finish(false);
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('keydown', esc, true);
  }

  function track(ev: PointerEvent) {
    if (!drag || !scroller) return;
    const rect = scroller.getBoundingClientRect();
    if (ev.clientY < rect.top + 24) scroller.scrollTop -= 12;
    else if (ev.clientY > rect.bottom - 24) scroller.scrollTop += 12;
    const y = ev.clientY - rect.top + scroller.scrollTop;
    const keys = new Set(drag.keys);
    let drop: Drop | null = null;
    let tail = false;
    if (
      ev.clientX >= rect.left &&
      ev.clientX <= rect.right &&
      ev.clientY >= rect.top &&
      ev.clientY <= rect.bottom
    ) {
      const idx = Math.floor(y / ROW);
      const row = rows[idx];
      if (!row) {
        const roots = siblings(tree.data, null, keys);
        drop = {
          parent: null,
          after: roots[roots.length - 1] ?? null,
          rowIndex: -1,
          zone: 'after'
        };
        tail = true;
      } else {
        const frac = (y - idx * ROW) / ROW;
        const zone =
          row.kind === 'folder'
            ? frac < 0.25
              ? 'before'
              : frac > 0.75
                ? 'after'
                : 'inside'
            : frac < 0.5
              ? 'before'
              : 'after';
        drop = resolveDrop(tree.data, rows, idx, zone, keys);
      }
    }
    drag = { ...drag, x: ev.clientX, y: ev.clientY, drop, tail };
    // Auto-expand a collapsed folder after 600 ms of hovering inside it.
    const hovered = drop?.zone === 'inside' ? rows[drop.rowIndex] : undefined;
    const hk = hovered && hovered.kind === 'folder' && !hovered.expanded ? hovered.key : null;
    if (hk !== hoverKey) {
      clearTimeout(hoverTimer);
      hoverKey = hk;
      if (hk && hovered)
        hoverTimer = setTimeout(() => void tree.setExpanded(hovered.id, true), 600);
    }
  }

  const indicator = $derived.by(() => {
    const d = drag?.drop;
    if (!d) return null;
    if (d.rowIndex < 0) return { top: rows.length * ROW, left: 8, inside: false };
    const row = rows[d.rowIndex];
    if (!row) return null;
    const child = d.zone === 'after' && row.kind === 'folder' && row.expanded && row.hasChildren;
    const depth = row.depth + (child ? 1 : 0);
    return {
      top: (d.rowIndex + (d.zone === 'before' ? 0 : 1)) * ROW,
      left: 8 + depth * INDENT,
      inside: d.zone === 'inside'
    };
  });
  const insideRow = $derived(drag?.drop?.zone === 'inside' ? drag.drop.rowIndex : -1);
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class="tree-wrap">
    <div
      bind:this={scroller}
      bind:clientHeight={viewH}
      class="tree"
      role="tree"
      aria-label={m.sidebar_hosts()}
      aria-multiselectable="true"
      tabindex="0"
      aria-activedescendant={tree.cursor ? 'ti-' + tree.cursor : undefined}
      onscroll={() => (scrollTop = scroller?.scrollTop ?? 0)}
      onkeydown={onKeyDown}
      oncontextmenu={(e) => onContextMenu(e, null)}
      onfocus={() => {
        if (!tree.cursor && rows[0]) tree.cursor = rows[0].key;
      }}
    >
      <div class="canvas" style:height="{rows.length * ROW}px">
        {#each visible as row, n (row.key)}
          {@const i = start + n}
          {@const sel = tree.selected.has(row.key)}
          <div
            id={'ti-' + row.key}
            class="row"
            class:sel
            class:cur={tree.cursor === row.key}
            class:into={insideRow === i}
            class:dragging={drag?.keys.includes(row.key)}
            role="treeitem"
            tabindex="-1"
            aria-selected={sel}
            aria-level={row.depth + 1}
            aria-expanded={row.kind === 'folder' ? row.expanded : undefined}
            style:top="{i * ROW}px"
            style:padding-left="{8 + row.depth * INDENT}px"
            onclick={(e) => onRowClick(e, row)}
            ondblclick={() => row.kind === 'host' && connectHost(row.id)}
            onkeydown={null}
            oncontextmenu={(e) => onContextMenu(e, row)}
            onpointerdown={(e) => onPointerDown(e, row)}
          >
            <TreeRowContent {row} oncommit={(v) => commitRename(row, v)} oncancel={cancelRename} />
          </div>
        {/each}
        {#if indicator && !indicator.inside}
          <div
            class="drop-line"
            style:top="{indicator.top}px"
            style:left="{indicator.left}px"
          ></div>
        {/if}
      </div>
      {#if tree.loaded && rows.length === 0}
        <EmptyState icon={Server} title={m.tree_empty()}>
          <button class="btn btn-primary btn-sm" onclick={() => ui.openHostEditor(null)}
            >{m.action_new_host()}</button
          >
        </EmptyState>
      {/if}
    </div>
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="menu"
      ><TreeMenu row={ctx.row} keys={ctx.keys} /></ContextMenu.Content
    >
  </ContextMenu.Portal>
</ContextMenu.Root>

{#if drag}
  <div class="tree-ghost" style:left="{drag.x + 12}px" style:top="{drag.y + 8}px">
    {drag.keys.length > 1
      ? m.drag_items({ count: drag.keys.length })
      : (tree.rows.find((r) => r.key === drag?.keys[0])?.label ?? '')}
  </div>
{/if}
