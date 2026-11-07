/** SFTP layout the user adjusts by dragging: pane split and column widths (kept per device). */
export type ColKey = 'size' | 'mtime' | 'mode';

const KEY = 'ibc.sftp.layout';
const LIMITS: Record<ColKey, [number, number, number]> = {
  size: [48, 220, 76],
  mtime: [72, 260, 142],
  mode: [56, 180, 96]
};
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

class SftpLayout {
  /** Left pane share, 0.2–0.8. */
  split = $state(0.5);
  cols = $state<Record<ColKey, number>>({ size: 76, mtime: 142, mode: 96 });

  constructor() {
    try {
      const raw = JSON.parse(localStorage.getItem(KEY) ?? 'null') as {
        split?: unknown;
        cols?: Record<string, unknown>;
      } | null;
      if (typeof raw?.split === 'number') this.split = clamp(raw.split, 0.2, 0.8);
      for (const k of Object.keys(LIMITS) as ColKey[]) {
        const v = raw?.cols?.[k];
        if (typeof v === 'number') this.cols[k] = clamp(v, LIMITS[k][0], LIMITS[k][1]);
      }
    } catch {
      /* storage unavailable: defaults */
    }
  }

  setSplit(v: number): void {
    this.split = clamp(v, 0.2, 0.8);
  }

  setCol(k: ColKey, px: number): void {
    this.cols[k] = clamp(Math.round(px), LIMITS[k][0], LIMITS[k][1]);
  }

  resetCol(k: ColKey): void {
    this.cols[k] = LIMITS[k][2];
    this.save();
  }

  save(): void {
    try {
      localStorage.setItem(KEY, JSON.stringify({ split: this.split, cols: this.cols }));
    } catch {
      /* ignore */
    }
  }
}

export const sftpLayout = new SftpLayout();

/** Pointer drag helper: calls `onMove(dx)` relative to the press point, `onEnd` on release. */
export function drag(e: PointerEvent, onMove: (dx: number) => void, onEnd: () => void): void {
  e.preventDefault();
  e.stopPropagation();
  const x0 = e.clientX;
  const move = (ev: PointerEvent) => onMove(ev.clientX - x0);
  const up = () => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    document.body.style.cursor = '';
    onEnd();
  };
  document.body.style.cursor = 'col-resize';
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
}
