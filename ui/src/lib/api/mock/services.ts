/** Mock: Docker containers, certificates and the menu-bar icon (ADR-012, ADR-013). */
import type { Commands, Container, TrayMenu } from '../types';
import { delay, ok } from './state';

const MIB = 1024 ** 2;
const DAY = 86_400_000;

const containers: Container[] = [
  [
    'a1b2c3d4e5f6',
    'xray',
    'teddysun/xray:1.8.24',
    'running',
    'Up 12 days',
    '0.0.0.0:443->443/tcp, :::443->443/tcp',
    3.2,
    48
  ],
  [
    'b2c3d4e5f6a1',
    'nginx',
    'nginx:1.27-alpine',
    'running',
    'Up 12 days (healthy)',
    '0.0.0.0:80->80/tcp',
    0.4,
    9
  ],
  [
    'c3d4e5f6a1b2',
    'postgres',
    'postgres:16',
    'running',
    'Up 3 days (healthy)',
    '127.0.0.1:5432->5432/tcp',
    6.8,
    412
  ],
  [
    'd4e5f6a1b2c3',
    'grafana',
    'grafana/grafana:11.2.0',
    'running',
    'Up 3 days',
    '127.0.0.1:3000->3000/tcp',
    1.1,
    96
  ],
  ['e5f6a1b2c3d4', 'backup', 'restic/restic:0.17.1', 'exited', 'Exited (0) 5 hours ago', '', 0, 0]
].map(([id, name, image, state, status, ports, cpu, mib]) => ({
  id: id as string,
  name: name as string,
  image: image as string,
  state: state as string,
  status: status as string,
  ports: ports as string,
  cpu_pct: state === 'running' ? (cpu as number) : null,
  mem_bytes: state === 'running' ? (mib as number) * MIB : null,
  mem_limit: state === 'running' ? 16 * 1024 * MIB : null
}));

export const dockerList: Commands['dockerList'] = async () => {
  await delay(500);
  return ok({
    access: 'Ok',
    containers: containers.map((c) =>
      c.cpu_pct == null ? c : { ...c, cpu_pct: c.cpu_pct * (0.6 + Math.random() * 0.8) }
    )
  });
};

const SAMPLE = [
  (n: string) => `91.204.17.6 - - "GET /api/v1/stats HTTP/1.1" 200 812 "-" "${n}-client/2.1"`,
  () => `176.59.40.12 - - "POST /login HTTP/1.1" 401 64 "-" "curl/8.5.0"`,
  () => `level=info msg="accepted connection" from=85.140.3.77:51244 inbound=vless-in`,
  () => `\x1b[32mINFO\x1b[0m  worker ready in \x1b[1m42 ms\x1b[0m`,
  () => `level=warn msg="slow upstream" upstream=10.0.0.5:5432 took=1.8s`,
  () => `66.249.66.1 - - "GET /robots.txt HTTP/1.1" 404 0 "-" "Googlebot/2.1"`,
  () => `\x1b[33mWARN\x1b[0m  certificate for vpn.example.net expires in 6 days`,
  () => `ERROR upstream timed out (110: Connection timed out) while reading response header`,
  () => `140.82.121.4 - - "GET /health HTTP/1.1" 502 157 "-" "uptime-bot"`,
  () => `\x1b[36mDEBUG\x1b[0m cache hit ratio=0.94 keys=18233`
];

export const dockerLogs: Commands['dockerLogs'] = async (_sid, id, lines) => {
  await delay(300);
  const name = containers.find((c) => c.id === id)?.name ?? id;
  const now = Date.now();
  const n = Math.min(lines, 400);
  const out = Array.from({ length: n }, (_, i) => {
    const t = new Date(now - (n - i) * 7000).toISOString();
    return `${t} ${SAMPLE[(i * 7) % SAMPLE.length]!(name)}`;
  });
  return ok(out.join('\n') + '\n');
};

export const dockerAction: Commands['dockerAction'] = async (_sid, id, action) => {
  await delay(900);
  const c = containers.find((x) => x.id === id);
  if (c) {
    c.state = action === 'Stop' ? 'exited' : 'running';
    c.status = action === 'Stop' ? 'Exited (0) less than a second ago' : 'Up 1 second';
    c.cpu_pct = action === 'Stop' ? null : 0.5;
    c.mem_bytes = action === 'Stop' ? null : 20 * MIB;
  }
  return ok(null);
};

const GB = 1024 * MIB;

export const dockerCompose: Commands['dockerCompose'] = async () => {
  await delay(400);
  const svc = (service: string, container: string, state: string, status: string) => ({
    service,
    container,
    state,
    status
  });
  return ok({
    access: 'Ok',
    projects: [
      {
        name: 'shop',
        dir: '/srv/shop',
        running: 3,
        total: 4,
        services: [
          svc('db', 'shop-db-1', 'running', 'Up 3 days (healthy)'),
          svc('redis', 'shop-redis-1', 'running', 'Up 3 days'),
          svc('web', 'shop-web-1', 'running', 'Up 3 days'),
          svc('worker', 'shop-worker-1', 'exited', 'Exited (1) 2 hours ago')
        ]
      },
      {
        name: 'monitoring',
        dir: '/opt/monitoring',
        running: 2,
        total: 2,
        services: [
          svc('grafana', 'monitoring-grafana-1', 'running', 'Up 3 days'),
          svc('prometheus', 'monitoring-prometheus-1', 'running', 'Up 3 days')
        ]
      }
    ]
  });
};

export const dockerImages: Commands['dockerImages'] = async () => {
  await delay(400);
  const img = (id: string, name: string, size: number, created: string, containers: number) => ({
    id,
    name,
    size_bytes: size,
    created,
    containers
  });
  return ok({
    access: 'Ok',
    images: [
      img('9f8e7d6c5b4a', 'postgres:16', 438 * MIB, '3 weeks ago', 1),
      img('1a2b3c4d5e6f', 'grafana/grafana:11.2.0', 466 * MIB, '2 months ago', 1),
      img('2b3c4d5e6f7a', 'teddysun/xray:1.8.24', 59 * MIB, '5 weeks ago', 1),
      img('3c4d5e6f7a8b', 'nginx:1.27-alpine', 48 * MIB, '6 weeks ago', 1),
      img('4d5e6f7a8b9c', 'restic/restic:0.17.1', 31 * MIB, '3 months ago', 1),
      img('5e6f7a8b9c0d', 'nginx:1.25', 187 * MIB, '9 months ago', 0),
      img('6f7a8b9c0d1e', '<none>', 1.2 * GB, '1 year ago', 0)
    ]
  });
};

export const dockerVolumes: Commands['dockerVolumes'] = async () => {
  await delay(500);
  return ok({
    access: 'Ok',
    volumes: [
      { name: 'shop_pgdata', driver: 'local', size_bytes: 412 * MIB, containers: 1 },
      { name: 'grafana-storage', driver: 'local', size_bytes: 96 * MIB, containers: 1 },
      { name: 'restic-cache', driver: 'local', size_bytes: 2.4 * GB, containers: 1 },
      { name: 'old_uploads', driver: 'local', size_bytes: 1.1 * GB, containers: 0 },
      { name: 'a3f1c9d27b6e', driver: 'local', size_bytes: null, containers: 0 }
    ]
  });
};

export const dockerNetworks: Commands['dockerNetworks'] = async () => {
  await delay(400);
  const net = (id: string, name: string, driver: string, subnet: string, containers: number) => ({
    id,
    name,
    driver,
    scope: 'local',
    subnet,
    containers
  });
  return ok({
    access: 'Ok',
    networks: [
      net('a1a1a1a1a1a1', 'bridge', 'bridge', '172.17.0.0/16', 3),
      net('b2b2b2b2b2b2', 'shop_default', 'bridge', '172.20.0.0/16', 4),
      net('c3c3c3c3c3c3', 'monitoring_default', 'bridge', '172.21.0.0/16', 2),
      net('d4d4d4d4d4d4', 'host', 'host', '', 0),
      net('e5e5e5e5e5e5', 'none', 'null', '', 0)
    ]
  });
};

const EVENT_SAMPLE: [string, string, string][] = [
  ['container', 'start', 'shop-worker-1'],
  ['container', 'die', 'shop-worker-1'],
  ['container', 'oom', 'shop-worker-1'],
  ['container', 'restart', 'nginx'],
  ['image', 'pull', 'postgres:16'],
  ['network', 'connect', 'shop_default'],
  ['volume', 'create', 'old_uploads'],
  ['container', 'stop', 'backup']
];

export const dockerEvents: Commands['dockerEvents'] = async (_sid, windowS) => {
  await delay(250);
  const now = Date.now();
  const n = Math.min(EVENT_SAMPLE.length * 2, Math.floor(windowS / 120));
  return ok({
    access: 'Ok',
    events: Array.from({ length: n }, (_, i) => {
      const [kind, action, name] = EVENT_SAMPLE[i % EVENT_SAMPLE.length]!;
      return { time: now - (i * 97 + 12) * 1000, kind, action, name };
    })
  });
};

export const certList: Commands['certList'] = async () => {
  await delay(400);
  const now = Date.now();
  return ok({
    certs: [
      {
        path: '/root/cert/vpn.example.net/fullchain.pem',
        names: ['vpn.example.net'],
        issuer: "Let's Encrypt",
        not_after: now + 6 * DAY
      },
      {
        path: '/etc/letsencrypt/live/example.net/cert.pem',
        names: ['example.net', 'www.example.net'],
        issuer: "Let's Encrypt",
        not_after: now + 19 * DAY
      },
      {
        path: '/etc/nginx/ssl/panel.pem',
        names: ['panel.example.net'],
        issuer: 'ZeroSSL',
        not_after: now + 74 * DAY
      }
    ],
    denied: 0,
    no_openssl: false
  });
};

/** The last menu the UI sent, for tests and the dev console. */
export let lastTray: TrayMenu | null = null;
export const traySet: Commands['traySet'] = async (menu) => {
  lastTray = menu;
  return ok(null);
};
