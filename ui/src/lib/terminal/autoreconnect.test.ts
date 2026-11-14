import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AutoReconnect, BACKOFF, retriableFailure, retriableReason } from './autoreconnect.svelte';

describe('auto reconnect', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('retries only what a new attempt can fix', () => {
    for (const r of [
      'network:ConnectionReset',
      'keepalive',
      'server:ByApplication',
      'connection closed'
    ])
      expect(retriableReason(r)).toBe(true);
    for (const r of ['closed', 'shell_exited', 'protocol', 'inactivity', null])
      expect(retriableReason(r)).toBe(false);
    expect(retriableFailure('Network')).toBe(true);
    expect(retriableFailure('AuthFailed')).toBe(false);
    expect(retriableFailure('HostKeyChanged')).toBe(false);
  });

  it('waits the backoff, counts down and runs', () => {
    const run = vi.fn();
    const a = new AutoReconnect(run);
    expect(a.schedule()).toBe(true);
    expect(a.waiting).toBe(true);
    expect(a.seconds).toBe(BACKOFF[0]);
    vi.advanceTimersByTime(BACKOFF[0] * 1000);
    expect(run).toHaveBeenCalledTimes(1);
    expect(a.attempt).toBe(1);
    expect(a.waiting).toBe(false);
    a.schedule();
    vi.advanceTimersByTime(1000);
    expect(a.seconds).toBe(BACKOFF[1] - 1);
  });

  it('gives up after the last pause and resets on cancel', () => {
    const run = vi.fn();
    const a = new AutoReconnect(run);
    for (let i = 0; i < BACKOFF.length; i++) {
      expect(a.schedule()).toBe(true);
      a.now();
    }
    expect(run).toHaveBeenCalledTimes(BACKOFF.length);
    expect(a.schedule()).toBe(false);
    expect(a.attempt).toBe(0);
    a.schedule();
    a.cancel();
    vi.advanceTimersByTime(60_000);
    expect(run).toHaveBeenCalledTimes(BACKOFF.length);
  });

  it('tries at once when the network comes back', () => {
    // Minimal window / document: the test runs in Node.
    const win = new EventTarget();
    const doc = Object.assign(new EventTarget(), { visibilityState: 'visible' });
    vi.stubGlobal('window', win);
    vi.stubGlobal('document', doc);
    vi.stubGlobal('navigator', { onLine: true });
    try {
      const run = vi.fn();
      const a = new AutoReconnect(run);
      a.schedule();
      win.dispatchEvent(new Event('online'));
      expect(run).toHaveBeenCalledTimes(1);
      a.schedule();
      doc.dispatchEvent(new Event('visibilitychange'));
      expect(run).toHaveBeenCalledTimes(2);
      a.schedule();
      a.cancel();
      win.dispatchEvent(new Event('online'));
      expect(run).toHaveBeenCalledTimes(2);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
