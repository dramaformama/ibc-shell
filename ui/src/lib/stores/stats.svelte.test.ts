import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { HostStats } from '$lib/api';

const { sample } = vi.hoisted(() => ({ sample: vi.fn() }));
vi.mock('$lib/api', () => ({ api: { statsSample: sample } }));

const snapshot: HostStats = {
  cpu_pct: null,
  cores: 1,
  cpu_cores: [null],
  cpu_user: null,
  cpu_system: null,
  cpu_nice: null,
  cpu_iowait: null,
  cpu_steal: null,
  load1: null,
  load5: 0,
  load15: 1,
  mem_total: 100,
  mem_used: 25,
  mem_cached: 10,
  mem_free: 5,
  swap_total: 0,
  swap_used: 0,
  disk_total: 100,
  disk_used: 50,
  net_rx_bps: 0,
  net_tx_bps: 0,
  ifaces: [],
  disk_read_bps: 0,
  disk_write_bps: 0,
  uptime_s: 60,
  os_name: null,
  temp_c: null,
  procs: [],
  listen: [],
  flows: [],
  conns: 0,
  full: true
};

describe('visible metrics polling', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();
    sample.mockReset();
    sample.mockResolvedValue({ status: 'ok', data: snapshot });
  });
  afterEach(() => vi.useRealTimers());

  it('shares a single poll when views change during a pending sample', async () => {
    const { stats } = await import('./stats.svelte');
    let finish!: (value: unknown) => void;
    sample.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        })
    );
    const stopInspector = stats.watch('session');
    stopInspector();
    const stopCard = stats.watch('session');
    expect(sample).toHaveBeenCalledTimes(1);
    finish({ status: 'ok', data: snapshot });
    await vi.advanceTimersByTimeAsync(0);
    expect(stats.get('session')?.cpu).toEqual([null]);
    expect(stats.get('session')?.mem).toEqual([25]);
    stopInspector(); // Cleanup must be idempotent; the card is still watching.
    await vi.advanceTimersByTimeAsync(4000);
    expect(sample).toHaveBeenCalledTimes(2);
    stopCard();
    await vi.advanceTimersByTimeAsync(8000);
    expect(sample).toHaveBeenCalledTimes(2);
    // History survives closing the panel, so reopening it shows the chart at once.
    expect(stats.get('session')?.mem).toEqual([25, 25]);
  });

  it('marks the last sample stale on a transport failure without inventing zeroes', async () => {
    const { stats } = await import('./stats.svelte');
    sample
      .mockResolvedValueOnce({ status: 'ok', data: snapshot })
      .mockRejectedValueOnce(new Error('transport unavailable'));
    const stop = stats.watch('stale');
    await vi.advanceTimersByTimeAsync(0);
    expect(stats.get('stale')?.stale).toBe(false);
    await vi.advanceTimersByTimeAsync(4000);
    expect(stats.get('stale')?.stale).toBe(true);
    expect(stats.get('stale')?.latest?.mem_used).toBe(25);
    expect(stats.get('stale')?.cpu).toEqual([null]);
    stop();
    await vi.advanceTimersByTimeAsync(8000);
  });

  it('stops after Unsupported and retries when the view is opened again', async () => {
    const { stats } = await import('./stats.svelte');
    sample.mockResolvedValueOnce({ status: 'error', error: { code: 'Unsupported' } });
    const stop = stats.watch('other');
    await vi.advanceTimersByTimeAsync(10000);
    expect(stats.get('other')?.unsupported).toBe(true);
    expect(sample).toHaveBeenCalledTimes(1);
    stop();
    const stopAgain = stats.watch('other');
    await vi.advanceTimersByTimeAsync(0);
    expect(sample).toHaveBeenCalledTimes(2);
    expect(stats.get('other')?.unsupported).toBe(false);
    stopAgain();
    await vi.advanceTimersByTimeAsync(2000);
  });
});
