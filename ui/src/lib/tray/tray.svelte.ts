/**
 * Menu-bar icon (ADR-013): a line per open or "connect at start" server with its state and load,
 * rebuilt from the stores and sent to the backend at most once a second when it changes.
 */
import {
  api,
  apiEvents,
  type HostId,
  type HostStats,
  type TrayItem,
  type TrayMenu
} from '$lib/api';
import * as m from '$lib/paraglide/messages.js';
import type { TabPhase } from '$lib/stores/sessions.svelte';

export interface TrayEntry {
  hostId: HostId;
  label: string;
  /** `null` = not open in the app (watched only). */
  phase: TabPhase | null;
  /** Live metrics, when sampled. */
  d: HostStats | null;
}

const pct = (v: number) => `${Math.round(v)}%`;

/** One menu line: a coloured dot, the name and what matters right now. */
export function line(e: TrayEntry): string {
  if (e.phase === 'connected' && e.d) {
    const mem = e.d.mem_total ? (e.d.mem_used / e.d.mem_total) * 100 : 0;
    const disk = e.d.disk_total ? (e.d.disk_used / e.d.disk_total) * 100 : 0;
    const cpu = e.d.cpu_pct ?? 0;
    const hot = cpu >= 90 || mem >= 90 || disk >= 90;
    const parts = [`CPU ${pct(cpu)}`, `RAM ${pct(mem)}`];
    if (disk >= 85) parts.push(`${m.tray_disk()} ${pct(disk)}`);
    return `${hot ? '🟠' : '🟢'} ${e.label} — ${parts.join(' · ')}`;
  }
  if (e.phase === 'connected') return `🟢 ${e.label}`;
  if (e.phase === 'connecting') return `🟡 ${e.label} — ${m.tray_connecting()}`;
  if (e.phase === 'failed') return `🔴 ${e.label} — ${m.tray_failed()}`;
  return `⚪ ${e.label} — ${m.tray_offline()}`;
}

/** The whole menu for the given servers (locked = no server lines). */
export function buildMenu(entries: TrayEntry[], locked: boolean): TrayMenu {
  const sorted = [...entries].sort((a, b) => a.label.localeCompare(b.label));
  const online = entries.filter((e) => e.phase === 'connected').length;
  const items: TrayItem[] = locked
    ? []
    : sorted.length
      ? sorted.slice(0, 24).map((e) => ({ host_id: e.hostId, text: line(e) }))
      : [{ host_id: null, text: m.tray_none() }];
  return {
    tooltip: locked ? 'iBC Shell' : `iBC Shell — ${m.tray_online({ count: online })}`,
    items,
    open_label: m.tray_open(),
    quit_label: m.tray_quit()
  };
}

let sent = '';
let timer: ReturnType<typeof setTimeout> | null = null;
let pending: TrayMenu | null = null;

/** Sends `menu` (or hides the icon with `null`), coalesced to one call per second. */
export function pushTray(menu: TrayMenu | null): void {
  const key = JSON.stringify(menu);
  if (key === sent && !timer) return;
  pending = menu;
  if (timer) return;
  timer = setTimeout(() => {
    timer = null;
    const k = JSON.stringify(pending);
    if (k === sent) return;
    sent = k;
    void api.traySet(pending).catch(() => undefined);
  }, 1000);
}

/** Clicking a server line in the menu opens it. */
export function onTrayOpen(open: (id: HostId) => void): Promise<() => void> {
  return apiEvents.listen('trayOpenHost', (p) => open(p.host_id));
}
