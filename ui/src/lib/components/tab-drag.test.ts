import { describe, expect, it } from 'vitest';
import { landing, tearing } from './tab-drag.svelte';

// Three 100 px tabs with 2 px gaps: centres at 50, 152, 254.
const slots = ['a', 'b', 'c'].map((key, i) => ({ key, left: i * 102, width: 100 }));

describe('tab drag landing slot', () => {
  it('stays put until the centre passes a neighbour', () => {
    expect(landing(slots, 0, 50)).toBe(0);
    expect(landing(slots, 0, 151)).toBe(0);
    expect(landing(slots, 0, 153)).toBe(1);
    expect(landing(slots, 0, 300)).toBe(2);
  });

  it('moves left past neighbours', () => {
    expect(landing(slots, 2, 254)).toBe(2);
    expect(landing(slots, 2, 140)).toBe(1);
    expect(landing(slots, 2, 10)).toBe(0);
  });
});

describe('tab tear-off', () => {
  // Bar from y 0 to 40, x 0 to 600.
  const bar = { top: 0, bottom: 40, left: 0, right: 600 } as DOMRect;

  it('needs a clear pull away from the bar before tearing', () => {
    expect(tearing(bar, 100, 60, false)).toBe(false);
    expect(tearing(bar, 100, 69, false)).toBe(true);
    expect(tearing(bar, 100, -29, false)).toBe(true);
    // Sideways past the end of the bar is still reordering.
    expect(tearing(bar, 900, 20, false)).toBe(false);
  });

  it('comes back only over the bar itself', () => {
    expect(tearing(bar, 100, 50, true)).toBe(true);
    expect(tearing(bar, 100, 30, true)).toBe(false);
    expect(tearing(bar, 900, 20, true)).toBe(true);
  });
});
