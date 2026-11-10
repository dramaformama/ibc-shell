import { describe, expect, it } from 'vitest';
import { landing } from './tab-drag.svelte';

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
