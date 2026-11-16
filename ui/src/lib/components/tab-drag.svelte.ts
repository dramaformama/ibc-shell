/**
 * Pointer-driven reordering of the tab bar (HTML5 drag-and-drop is unreliable in the
 * webview). The lifted tab follows the pointer; the others slide by its width to show where it
 * lands; on release the order is committed and the offsets are dropped without animating.
 *
 * Pulled away from the bar the tab tears off: its slot stays as a faint placeholder and a
 * floating copy follows the pointer (`ghost`; past the window edge the owner shows a native one
 * through `tear.move`). Releasing a torn tab calls `tear.drop` instead of reordering.
 */

/** Movement before a press becomes a drag (a plain click must stay a click). */
const THRESHOLD = 4;
/** Distance from the bar, above or below, at which the tab tears off. */
const TEAR = 28;

/** What the bar does with a torn-off tab. */
export interface TearOff {
  /** Whether this group may leave the bar at all. */
  allowed(key: string): boolean;
  /** Tab torn off (true) or put back into the bar (false). */
  change(key: string, torn: boolean): void;
  /** Pointer moved while torn; `outside` = beyond this window. */
  move(e: PointerEvent, outside: boolean): void;
  /** Released while torn. */
  drop(key: string, e: PointerEvent): void;
  /** Drag over (drop or cancel). */
  end(): void;
}

/** True when the pointer is past the bar far enough to tear the tab off. */
export function tearing(bar: DOMRect, x: number, y: number, torn: boolean): boolean {
  // Once torn, the tab comes back only when the pointer is over the bar itself.
  const margin = torn ? 0 : TEAR;
  return (
    y < bar.top - margin || y > bar.bottom + margin || (torn && (x < bar.left || x > bar.right))
  );
}

function outsideWindow(e: PointerEvent): boolean {
  return e.clientX < 0 || e.clientY < 0 || e.clientX > innerWidth || e.clientY > innerHeight;
}

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
  /** Pulled out of the bar. */
  torn = $state(false);
  /** Floating copy position inside this window (client px), null when hidden. */
  ghost = $state<{ x: number; y: number } | null>(null);
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
    private tear: TearOff
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
    const key = this.key;
    if (key !== null && this.tear.allowed(key)) {
      const torn = tearing(this.bounds(), e.clientX, e.clientY, this.torn);
      if (torn !== this.torn) {
        this.torn = torn;
        this.tear.change(key, torn);
      }
    }
    if (this.torn) {
      const outside = outsideWindow(e);
      this.ghost = outside ? null : { x: e.clientX, y: e.clientY };
      this.#dx = 0;
      this.#to = this.#from;
      this.tear.move(e, outside);
      return;
    }
    this.ghost = null;
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
    if (this.key !== null) this.tear.end();
    this.settling = true;
    this.torn = false;
    this.ghost = null;
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
    if (this.torn) this.tear.drop(key, e);
    else this.commit(key, this.#to);
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
    if (key === this.key) return this.torn ? undefined : `translateX(${this.#dx}px)`;
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
