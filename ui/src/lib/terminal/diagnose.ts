/**
 * Turns "which step was running and what the backend said" into one plain explanation.
 * Pure: the component maps a `kind` to localized text. Inputs are error codes and IO error
 * kinds only (never server-provided text).
 */
import type { ApiError, ConnectStep } from '$lib/api';

export interface Attempt {
  addr: string;
  /** `null` = connected; else `Timeout` or an IO error kind. */
  failure: string | null;
}

export type DiagKind =
  | 'dns'
  | 'dns_timeout'
  | 'refused'
  | 'unreachable'
  | 'tcp_timeout'
  | 'tcp_other'
  | 'jump'
  | 'hs_timeout'
  | 'hs_dropped'
  | 'hs_algo'
  | 'hs_protocol'
  | 'auth'
  | 'channel'
  | 'cancelled'
  | 'no_user'
  | 'no_host'
  | 'prepare'
  | 'generic';

export interface Diagnosis {
  kind: DiagKind;
  /** The address that was tried last, when there is one. */
  addr: string | null;
  /** The detail the explanation quotes (algorithm kind, IO kind). */
  detail: string;
  /** Methods the server still accepts (auth). */
  methods: string[];
}

export interface DiagInput {
  /** The step that was running when it failed; `null` = before any step. */
  step: ConnectStep | null;
  error: ApiError;
  attempts: Attempt[];
  /** The failing hop is not the first one: the previous host opened the tunnel. */
  viaJump: boolean;
}

const ALGO = new Set(['Kex', 'Key', 'Cipher', 'Mac', 'Compression']);
const UNREACHABLE = new Set(['HostUnreachable', 'NetworkUnreachable']);
const DROPPED = new Set([
  'disconnected',
  'ConnectionReset',
  'ConnectionAborted',
  'BrokenPipe',
  'UnexpectedEof'
]);

export function diagnose({ step, error, attempts, viaJump }: DiagInput): Diagnosis {
  const addr = attempts.at(-1)?.addr ?? null;
  const detail = error.code === 'Network' ? error.detail : '';
  const out = (kind: DiagKind, extra: Partial<Diagnosis> = {}): Diagnosis => ({
    kind,
    addr,
    detail,
    methods: [],
    ...extra
  });

  if (error.code === 'Cancelled') return out('cancelled');
  if (error.code === 'AuthFailed') return out('auth', { methods: error.detail.methods_left });
  if (step === null) {
    if (error.code === 'Validation') {
      if (error.detail.includes('username')) return out('no_user');
      if (error.detail.includes('hostname')) return out('no_host');
    }
    return out('prepare');
  }
  if (error.code !== 'Network' && error.code !== 'Timeout') return out('generic');
  const timeout = error.code === 'Timeout';

  switch (step) {
    case 'Resolving':
      return out(timeout ? 'dns_timeout' : 'dns');
    case 'Connecting':
      if (viaJump) return out('jump');
      if (timeout) return out('tcp_timeout');
      if (detail === 'ConnectionRefused') return out('refused');
      if (UNREACHABLE.has(detail)) return out('unreachable');
      return out('tcp_other');
    case 'Handshake':
      if (timeout) return out('hs_timeout');
      if (ALGO.has(detail)) return out('hs_algo');
      if (DROPPED.has(detail)) return out('hs_dropped');
      if (detail.startsWith('protocol')) return out('hs_protocol');
      return out('hs_dropped');
    case 'Authenticating':
      return out(timeout ? 'hs_timeout' : 'hs_dropped');
    case 'OpeningChannel':
      return out('channel');
  }
}
