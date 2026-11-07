import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { HostReachability, Result } from '$lib/api';
import { ReachabilityStore } from './reachability.svelte';

const { probe } = vi.hoisted(() => ({ probe: vi.fn() }));
vi.mock('$lib/api', () => ({ api: { hostProbe: probe } }));
const reply = (): Result<HostReachability> => ({
  status: 'ok',
  data: { reachable: true, latency_ms: 10, checked_at: Date.now() }
});

describe('bounded visible-host observations', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    probe.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  it('limits concurrency, skips invisible queued hosts, and shares watchers', async () => {
    const store = new ReachabilityStore();
    const pending: Array<() => void> = [];
    probe.mockImplementation(() => new Promise((resolve) => pending.push(() => resolve(reply()))));
    const stops = Array.from({ length: 8 }, (_, i) => store.watch(`host-${i}`));
    const stopDuplicate = store.watch('host-0');
    await vi.advanceTimersByTimeAsync(0);
    expect(probe).toHaveBeenCalledTimes(4);
    stops[4]!(); // Never probe this queued host after navigation.
    pending.splice(0).forEach((resolve) => resolve());
    await vi.advanceTimersByTimeAsync(0);
    expect(probe).toHaveBeenCalledTimes(7);
    expect(probe.mock.calls.some(([id]) => id === 'host-4')).toBe(false);
    pending.splice(0).forEach((resolve) => resolve());
    await vi.advanceTimersByTimeAsync(0);
    expect(store.data['host-0']?.reachable).toBe(true);
    stops.forEach((stop) => stop());
    stopDuplicate();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(probe).toHaveBeenCalledTimes(7);
  });

  it('checks immediately when returning after an invisible in-flight request finishes', async () => {
    const store = new ReachabilityStore();
    let finish!: (value: Result<HostReachability>) => void;
    probe.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        })
    );
    const stop = store.watch('returning');
    await vi.advanceTimersByTimeAsync(0);
    stop();
    finish(reply());
    await vi.advanceTimersByTimeAsync(0);
    probe.mockResolvedValue(reply());
    const stopAgain = store.watch('returning');
    await vi.advanceTimersByTimeAsync(0);
    expect(probe).toHaveBeenCalledTimes(2);
    expect(store.data.returning?.reachable).toBe(true);
    stopAgain();
    store.clear();
  });

  it('does not restore observations from a request that finishes after lock', async () => {
    const store = new ReachabilityStore();
    let finish!: (value: Result<HostReachability>) => void;
    probe.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        })
    );
    const stopOld = store.watch('host');
    await vi.advanceTimersByTimeAsync(0);
    store.clear();
    finish(reply());
    await vi.advanceTimersByTimeAsync(0);
    expect(store.data).toEqual({});
    probe.mockResolvedValue(reply());
    const stopNew = store.watch('host');
    stopOld();
    await vi.advanceTimersByTimeAsync(0);
    expect(store.data.host?.reachable).toBe(true);
    stopNew();
    store.clear();
  });
});
