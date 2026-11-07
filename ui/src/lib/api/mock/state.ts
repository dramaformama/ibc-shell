/**
 * In-memory database + seed data for the mock API.
 * Seed: 3 flagged folders (de / nl / fi), one nested folder, 12 hosts, 2 identities.
 */
import type {
  ApiError,
  Folder,
  FolderDefaults,
  Host,
  IdentityDto,
  KnownHostSummary,
  PortForward,
  Result,
  Snippet,
  Tag,
  Value
} from '../types';

export const ok = <T>(data: T): Result<T> => ({ status: 'ok', data });
export const err = <T = never>(error: ApiError): Result<T> => ({ status: 'error', error });
export const notFound = <T = never>(): Result<T> => err<T>({ code: 'NotFound' });
export const now = (): number => Date.now();
export const uuid = (): string => crypto.randomUUID();
/** Simulated IPC latency so loading states are visible. */
export const delay = (ms = 20): Promise<void> => new Promise((r) => setTimeout(r, ms));

/** Fractional key after `prev` (good enough for the mock: appends a character). */
export function keyAfter(prev: string | undefined): string {
  return prev ? `${prev}V` : 'a0';
}

const emptyDefaults = (): FolderDefaults => ({
  username: null,
  port: null,
  identity_id: null,
  jump_host_id: null,
  startup_cmd: null,
  keepalive_s: null,
  term_theme: null
});

function folder(
  id: string,
  name: string,
  cc: string | null,
  parent: string | null,
  key: string
): Folder {
  const t = now();
  return {
    id,
    parent_id: parent,
    name,
    country_code: cc,
    color: null,
    icon: null,
    sort_key: key,
    expanded: true,
    defaults: emptyDefaults(),
    created_at: t,
    updated_at: t
  };
}

function host(
  label: string,
  hostname: string,
  folderId: string | null,
  key: string,
  extra: Partial<Host> = {}
): Host {
  const t = now();
  return {
    id: uuid(),
    folder_id: folderId,
    label,
    hostname,
    port: null,
    username: null,
    identity_id: null,
    jump_host_id: null,
    country_code: null,
    startup_cmd: null,
    keepalive_s: null,
    term_theme: null,
    notes: null,
    tag_ids: [],
    sort_key: key,
    last_connected_at: null,
    created_at: t,
    updated_at: t,
    ...extra
  };
}

export const ID = {
  de: '01900000-0000-7000-8000-0000000000de',
  nl: '01900000-0000-7000-8000-0000000000a1',
  fi: '01900000-0000-7000-8000-0000000000f1',
  deDb: '01900000-0000-7000-8000-0000000000db',
  idKey: '01900000-0000-7000-8000-00000000ed25',
  idPw: '01900000-0000-7000-8000-00000000aa01',
  tagProd: '01900000-0000-7000-8000-00000000ba01'
} as const;

function seed() {
  const folders: Folder[] = [
    folder(ID.de, 'Germany', 'de', null, 'a0'),
    folder(ID.nl, 'Netherlands', 'nl', null, 'a1'),
    folder(ID.fi, 'Finland', 'fi', null, 'a2'),
    folder(ID.deDb, 'Databases', null, ID.de, 'a2')
  ];
  folders[0]!.defaults = { ...emptyDefaults(), username: 'root', identity_id: ID.idKey };
  const hosts: Host[] = [
    host('fra-1', '203.0.113.10', ID.de, 'a0', { tag_ids: [ID.tagProd] }),
    host('fra-2', '203.0.113.11', ID.de, 'a1'),
    host('fra-db-1', '10.0.0.5', ID.deDb, 'a0', { notes: 'Primary Postgres' }),
    host('fra-db-2', '10.0.0.6', ID.deDb, 'a1'),
    host('ams-1', '198.51.100.20', ID.nl, 'a0', { username: 'deploy', identity_id: ID.idPw }),
    host('ams-2', '198.51.100.21', ID.nl, 'a1'),
    host('ams-edge', 'edge.example.nl', ID.nl, 'a2', { port: 2222 }),
    host('hel-1 (new key)', '192.0.2.30', ID.fi, 'a0'),
    host('hel-2 (changed key)', '192.0.2.31', ID.fi, 'a1'),
    host('hel-otp (2FA)', '192.0.2.32', ID.fi, 'a2'),
    host('home-nas', 'nas.local', null, 'a3', { country_code: 'us', username: 'admin' }),
    host('pi', '192.168.1.50', null, 'a4', { username: 'pi' })
  ];
  const t = now();
  const identities: IdentityDto[] = [
    {
      id: ID.idKey,
      label: 'Work ed25519',
      username: 'root',
      kind: 'key',
      key_algo: 'ssh-ed25519',
      key_fingerprint: 'SHA256:9pN0mKq3xY7Lr2v1oQ8bTzW5cU4eA6sDfGhJkLmNoPq',
      public_key:
        'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockMockMockMockMockMockMockMockMockMock work',
      has_password: false,
      has_passphrase: true,
      created_at: t,
      updated_at: t
    },
    {
      id: ID.idPw,
      label: 'Deploy password',
      username: 'deploy',
      kind: 'password',
      key_algo: null,
      key_fingerprint: null,
      public_key: null,
      has_password: true,
      has_passphrase: false,
      created_at: t,
      updated_at: t
    }
  ];
  const tags: Tag[] = [{ id: ID.tagProd, name: 'prod', color: 'danger' }];
  const snippets: Snippet[] = [
    {
      id: uuid(),
      folder_id: null,
      name: 'Disk usage',
      body: 'df -h\n',
      sort_key: 'a0',
      created_at: t,
      updated_at: t
    },
    {
      id: uuid(),
      folder_id: ID.de,
      name: 'Tail syslog',
      body: 'tail -f /var/log/syslog\n',
      sort_key: 'a1',
      created_at: t,
      updated_at: t
    }
  ];
  const knownHosts: KnownHostSummary[] = [
    {
      id: uuid(),
      host_pattern: '203.0.113.10',
      key_type: 'ssh-ed25519',
      fingerprint: 'SHA256:AbCdEfGhIjKlMnOpQrStUvWxYz0123456789+/AbCd',
      added_at: t
    },
    {
      id: uuid(),
      host_pattern: '192.0.2.31',
      key_type: 'ssh-ed25519',
      fingerprint: 'SHA256:OldOldOldOldOldOldOldOldOldOldOldOldOldOldO',
      added_at: t
    }
  ];
  return {
    folders,
    hosts,
    identities,
    tags,
    snippets,
    forwards: [] as PortForward[],
    knownHosts,
    settings: new Map<string, Value>([
      ['theme', 'dark'],
      ['locale', 'en']
    ])
  };
}

/** The mutable mock database. `resetMockDb()` restores the seed (tests). */
export let db = seed();

export function resetMockDb(): void {
  db = seed();
}

/** Depth of a folder (root children = 1). */
export function folderDepth(id: string | null): number {
  let depth = 0;
  let cur = id;
  while (cur) {
    depth += 1;
    cur = db.folders.find((f) => f.id === cur)?.parent_id ?? null;
  }
  return depth;
}

/** True if `candidate` is `id` or one of its descendants. */
export function isSelfOrDescendant(id: string, candidate: string | null): boolean {
  let cur = candidate;
  while (cur) {
    if (cur === id) return true;
    cur = db.folders.find((f) => f.id === cur)?.parent_id ?? null;
  }
  return false;
}
