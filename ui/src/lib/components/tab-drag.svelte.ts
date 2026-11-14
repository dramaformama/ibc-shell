/**
 * Pointer-driven reordering of the tab bar (HTML5 drag-and-drop is unreliable in the
 * webview). The lifted tab follows the pointer; the others slide by its width to show where it
 * lands; on release the order is committed and the offsets are dropped without animating.
 */

/** Movement before a press becomes a drag (a plain click must stay a click). */
const THRESHOLD = 4;

interface Slot {
  key: string;
  left: number;
  width: number;
}

/** Index the dragged slot lands on, given the slots in order and its current centre. */
export function landing(slots: Slot[], from: number, center: number): number {
  let to = 0;
  slots.forEach((s, i) => {
    if (i !== from && s.left + s.width / 2 < center) to++;
  });
  return to;
}

export class TabDrag {
  /** Group being dragged (after the threshold). */
  key = $state<string | null>(null);
  /** One frame without transitions right after a drop. */
  settling = $state(false);
  #dx = $state(0);
  #to = $state(0);
  #slots: Slot[] = [];
  #from = -1;
  #x0 = 0;
  #y0 = 0;
  #pointer: number | null = null;
  #capture: HTMLElement | null = null;
  #pending: string | null = null;
  #swallow = false;
  constructor(
    private items: () => HTMLElement[],
    private commit: (key: string, to: number) => void,
    private bounds: () => DOMRect,
    private detach: (key: string) => boolean
  ) {}

  down(e: PointerEvent, key: string): void {
    this.#swallow = false;
    if (this.#pointer !== null || e.button !== 0 || e.pointerType === 'touch') return;
    this.#slots = this.items().map((el) => {
      const r = el.getBoundingClientRect();
      return { key: el.dataset.tabKey ?? '', left: r.left, width: r.width };
    });
    this.#from = this.#slots.findIndex((s) => s.key === key);
    if (this.#from === -1) return;
    this.#x0 = e.clientX;
    this.#y0 = e.clientY;
    this.#pointer = e.pointerId;
    this.#capture = e.currentTarget instanceof HTMLElement ? e.currentTarget : null;
    this.#pending = key;
    window.addEventListener('pointermove', this.#move);
    window.addEventListener('pointerup', this.#up);
    window.addEventListener('pointercancel', this.#cancel);
    window.addEventListener('lostpointercapture', this.#cancel);
    window.addEventListener('keydown', this.#key);
  }

  #move = (e: PointerEvent): void => {
    if (e.pointerId !== this.#pointer) return;
    const raw = e.clientX - this.#x0;
    if (this.key === null) {
      if (Math.hypot(raw, e.clientY - this.#y0) < THRESHOLD) return;
      this.#capture?.setPointerCapture(e.pointerId);
      this.key = this.#pending;
    }
    const me = this.#slots[this.#from]!;
    const first = this.#slots[0]!;
    const last = this.#slots.at(-1)!;
    // Keep the lifted tab inside the bar.
    const dx = Math.min(
      last.left + last.width - me.width - me.left,
      Math.max(first.left - me.left, raw)
    );
    this.#dx = dx;
    this.#to = landing(this.#slots, this.#from, me.left + me.width / 2 + dx);
  };

  #stop(): void {
    window.removeEventListener('pointermove', this.#move);
    window.removeEventListener('pointerup', this.#up);
    window.removeEventListener('pointercancel', this.#cancel);
    window.removeEventListener('lostpointercapture', this.#cancel);
    window.removeEventListener('keydown', this.#key);
    if (this.#pointer !== null && this.#capture?.hasPointerCapture(this.#pointer)) {
      this.#capture.releasePointerCapture(this.#pointer);
    }
    this.#pending = null;
    this.#pointer = null;
    this.#capture = null;
  }

  #reset(): void {
    this.settling = true;
    this.key = null;
    this.#dx = 0;
    requestAnimationFrame(() => requestAnimationFrame(() => (this.settling = false)));
  }

  #up = (e: PointerEvent): void => {
    if (e.pointerId !== this.#pointer) return;
    const key = this.key;
    this.#stop();
    if (key === null) return;
    this.#swallow = true;
    const r = this.bounds();
    const outside =
      e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
    if (!outside || !this.detach(key)) this.commit(key, this.#to);
    this.#reset();
  };

  #cancel = (e: PointerEvent): void => {
    if (e.pointerId === this.#pointer) this.#abort();
  };

  #key = (e: KeyboardEvent): void => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    this.#abort();
  };

  #abort = (): void => {
    this.#stop();
    if (this.key === null) return;
    this.#swallow = true;
    this.#reset();
  };

  /** CSS transform for a tab while dragging. */
  offset(key: string): string | undefined {
    if (this.key === null) return undefined;
    if (key === this.key) return `translateX(${this.#dx}px)`;
    const i = this.#slots.findIndex((s) => s.key === key);
    const me = this.#slots[this.#from];
    const next = this.#slots[this.#from + 1] ?? this.#slots[this.#from - 1];
    if (i === -1 || !me || !next) return undefined;
    const gap = Math.abs(next.left - me.left) - (next.left > me.left ? me.width : next.width);
    const step = me.width + gap;
    const to = this.#to;
    if (this.#from < to && i > this.#from && i <= to) return `translateX(${-step}px)`;
    if (to < this.#from && i >= to && i < this.#from) return `translateX(${step}px)`;
    return undefined;
  }

  /** True once after a drag: the click that ends it must not select the tab. */
  swallowClick(): boolean {
    const v = this.#swallow;
    this.#swallow = false;
    return v;
  }
}
