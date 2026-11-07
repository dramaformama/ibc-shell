/** Mock: plausible, slowly varying host metrics (8 cores, Ubuntu 24.04). */
import type { Commands, HostStats } from '../types';
import { db, delay, notFound, ok } from './state';

const GIB = 1024 ** 3;
const MIB = 1024 ** 2;
const t0 = Date.now();

/** Smooth wobble in [lo, hi] (sum of sines, no randomness) so graphs look continuous. */
function wave(lo: number, hi: number, periodS: number, phase = 0): number {
  const t = (Date.now() - t0) / 1000;
  const v =
    0.5 +
    0.35 * Math.sin((2 * Math.PI * t) / periodS + phase) +
    0.15 * Math.sin((2 * Math.PI * t) / (periodS / 3.7) + phase * 2);
  return lo + (hi - lo) * v;
}

const PROCS: Array<[number, string, number, number, string]> = [
  [1432, 'postgres', 14, 1.4, 'postgres'],
  [2210, 'www-data', 9, 0.4, 'nginx'],
  [981, 'root', 6, 0.9, 'dockerd'],
  [3304, 'deploy', 5, 0.7, 'node'],
  [1207, 'redis', 3, 0.2, 'redis-server'],
  [877, 'root', 2, 0.1, 'sshd'],
  [4102, 'deploy', 1.5, 0.3, 'python3'],
  [1, 'root', 0.4, 0.1, 'systemd']
];

/** inbound, port, peer, connections, MiB so far, MiB/s weight. */
const FLOWS: Array<[boolean, number, string, number, number, number]> = [
  [true, 443, '91.204.17.6', 6, 410, 0.9],
  [true, 443, '176.59.40.12', 4, 220, 0.5],
  [true, 443, '2a00:1fa0:4418::1', 3, 96, 0.3],
  [true, 22, '85.140.3.77', 1, 2, 0.01],
  [true, 80, '66.249.66.1', 2, 1, 0.02],
  [false, 443, '140.82.121.4', 2, 38, 0.05],
  [false, 53, '1.1.1.1', 1, 0.3, 0.001]
];

export const statsSample: Commands['statsSample'] = async () => {
  await delay(300);
  const cpu_cores = Array.from({ length: 8 }, (_, i) => wave(4, 85, 17 + i * 3, i));
  const cpu_pct = cpu_cores.reduce((a, b) => a + b, 0) / cpu_cores.length;
  const stats: HostStats = {
    cpu_pct,
    cores: 8,
    cpu_cores,
    cpu_core_split: cpu_cores.map((c, i) => ({
      user: c * 0.62,
      system: c * 0.26,
      nice: i % 3 === 0 ? c * 0.04 : 0,
      iowait: i % 4 === 1 ? Math.min(12, c * 0.15) : c * 0.03,
      steal: c * 0.02
    })),
    cpu_user: cpu_pct * 0.6,
    cpu_system: cpu_pct * 0.28,
    cpu_nice: 0,
    cpu_iowait: cpu_pct * 0.08,
    cpu_steal: cpu_pct * 0.04,
    load1: wave(0.4, 3.2, 40),
    load5: wave(0.8, 2.4, 90, 1),
    load15: wave(1, 2, 200, 2),
    mem_total: 16 * GIB,
    mem_used: Math.round(wave(6.2, 7.4, 120) * GIB),
    mem_cached: Math.round(wave(3.8, 4.4, 150, 1) * GIB),
    mem_free: Math.round(wave(2.1, 2.9, 170) * GIB),
    swap_total: 2 * GIB,
    swap_used: Math.round(wave(0.1, 0.3, 300) * GIB),
    disk_total: 240 * GIB,
    disk_used: 96 * GIB,
    net_rx_bps: Math.round(wave(0.1, 6, 23) * MIB),
    net_tx_bps: Math.round(wave(0.05, 2, 31, 2) * MIB),
    ifaces: [
      {
        name: 'eth0',
        rx_bps: Math.round(wave(0.1, 5.6, 23) * MIB),
        tx_bps: Math.round(wave(0.05, 1.8, 31, 2) * MIB),
        rx_total: 812 * GIB,
        tx_total: 96 * GIB
      },
      {
        name: 'docker0',
        rx_bps: Math.round(wave(0, 0.4, 41) * MIB),
        tx_bps: Math.round(wave(0, 0.2, 47, 1) * MIB),
        rx_total: 3.4 * GIB,
        tx_total: 2.1 * GIB
      }
    ],
    disk_read_bps: Math.round(wave(0, 12, 29) * MIB),
    disk_write_bps: Math.round(wave(0.1, 8, 37, 1) * MIB),
    uptime_s: 3 * 86400 + 4 * 3600 + Math.floor((Date.now() - t0) / 1000),
    os_name: 'Ubuntu 24.04 LTS',
    temp_c: wave(44, 58, 60),
    procs: PROCS.map(([pid, user, cpu, gib, command]) => ({
      pid,
      user,
      cpu: cpu * (0.7 + 0.6 * wave(0, 1, 19 + (pid % 13))),
      mem_bytes: Math.round(gib * GIB),
      command
    })),
    listen: [
      { proto: 'tcp', addr: '0.0.0.0', port: 22, process: 'sshd' },
      { proto: 'tcp', addr: '127.0.0.53', port: 53, process: 'systemd-resolve' },
      { proto: 'tcp', addr: '0.0.0.0', port: 80, process: 'nginx' },
      { proto: 'tcp', addr: '::', port: 443, process: 'xray' },
      { proto: 'tcp', addr: '127.0.0.1', port: 5432, process: 'postgres' },
      { proto: 'udp', addr: '0.0.0.0', port: 51820, process: null }
    ],
    flows: FLOWS.map(([inbound, port, peer, conns, mib, share]) => {
      const grow = (Date.now() - t0) / 1000;
      return {
        inbound,
        port,
        peer,
        conns,
        bytes_in: Math.round((mib + grow * share * 0.8) * MIB),
        bytes_out: Math.round((mib * 6 + grow * share * 5) * MIB)
      };
    }),
    conns: 23,
    full: true
  };
  return ok(stats);
};

/** Reachability is independent of whether a terminal tab is open. */
export const hostProbe: Commands['hostProbe'] = async (id) => {
  await delay(120);
  const host = db.hosts.find((h) => h.id === id);
  if (!host) return notFound();
  const reachable = !['home-nas', 'pi'].includes(host.label);
  return ok({
    reachable,
    latency_ms: reachable ? 12 + ((host.label.length * 7) % 48) : null,
    checked_at: Date.now()
  });
};
