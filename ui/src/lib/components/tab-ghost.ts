/**
 * Native floating tab for drags past the window edge (ADR-027). The page draws the copy while
 * the pointer is inside the window; outside it a tiny click-through window follows instead.
 */
import { isTauri } from '@tauri-apps/api/core';
import { api } from '$lib/api';
import { normalizeCode } from '$lib/flags/flags';

export class NativeGhost {
  #ready: Promise<boolean> | null = null;
  #frame = 0;
  #at: { x: number; y: number; visible: boolean } | null = null;
  #shown = false;

  /** Prepares the hidden ghost as soon as the tab tears off, so it is ready at the edge. */
  open(title: string, flag: string | null) {
    if (!isTauri() || this.#ready) return;
    const dark = document.documentElement.dataset.theme === 'dark';
    this.#ready = api
      .tabGhostOpen(title, normalizeCode(flag) ?? null, dark)
      .then((r) => r.status === 'ok')
      .catch(() => false);
  }

  /** Follows the pointer (one move per frame); hidden while the page draws the copy itself. */
  move(e: PointerEvent, outside: boolean) {
    if (!this.#ready) return;
    if (!outside && !this.#shown) return;
    this.#at = { x: e.screenX, y: e.screenY, visible: outside };
    if (this.#frame) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = 0;
      const at = this.#at;
      const ready = this.#ready;
      if (!at || !ready) return;
      this.#shown = at.visible;
      void ready.then((ok) => {
        if (ok) void api.tabGhostMove(at.x, at.y, at.visible);
      });
    });
  }

  close() {
    cancelAnimationFrame(this.#frame);
    this.#frame = 0;
    this.#at = null;
    this.#shown = false;
    const ready = this.#ready;
    this.#ready = null;
    void ready?.then((ok) => {
      if (ok) void api.tabGhostClose();
    });
  }
}
