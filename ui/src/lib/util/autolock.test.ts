import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AUTOLOCK_DEFAULT, parseAutolock, watchIdle, type IdleTarget } from './autolock';

describe('parseAutolock', () => {
  it('accepts sane numbers and falls back otherwise', () => {
    expect(parseAutolock(5)).toBe(5);
    expect(parseAutolock(0)).toBe(0);
    expect(parseAutolock(-1)).toBe(AUTOLOCK_DEFAULT);
    expect(parseAutolock('x')).toBe(AUTOLOCK_DEFAULT);
    expect(parseAutolock(NaN)).toBe(AUTOLOCK_DEFAULT);
    expect(parseAutolock(1e9)).toBe(AUTOLOCK_DEFAULT);
  });
});

describe('watchIdle', () => {
  const handlers = new Map<string, () => void>();
  const target: IdleTarget = {
    addEventListener: (t: string, h: unknown) => void handlers.set(t, h as () => void),
    removeEventListener: (t: string) => void handlers.delete(t)
  };
  beforeEach(() => {
    vi.useFakeTimers();
    handlers.clear();
  });
  afterEach(() => vi.useRealTimers());

  it('fires after the idle timeout', () => {
    const idle = vi.fn();
    watchIdle(1, idle, target);
    vi.advanceTimersByTime(59_000);
    expect(idle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1_000);
    expect(idle).toHaveBeenCalledOnce();
  });

  it('activity postpones it', () => {
    const idle = vi.fn();
    watchIdle(1, idle, target);
    vi.advanceTimersByTime(50_000);
    handlers.get('keydown')?.();
    vi.advanceTimersByTime(50_000);
    expect(idle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(10_000);
    expect(idle).toHaveBeenCalledOnce();
  });

  it('0 minutes never fires and stop cancels', () => {
    const idle = vi.fn();
    watchIdle(0, idle, target);
    const stop = watchIdle(1, idle, target);
    stop();
    vi.advanceTimersByTime(10 * 60_000);
    expect(idle).not.toHaveBeenCalled();
    expect(handlers.size).toBe(0);
  });
});
