import { describe, expect, it } from 'vitest';
import { diagnose, type Attempt, type DiagInput } from './diagnose';

const base = { attempts: [] as Attempt[], viaJump: false };
const net = (detail: string): DiagInput['error'] => ({ code: 'Network', detail });
const kind = (i: Partial<DiagInput> & Pick<DiagInput, 'step' | 'error'>) =>
  diagnose({ ...base, ...i }).kind;

describe('diagnose', () => {
  it('tells apart where the connection stopped', () => {
    expect(kind({ step: 'Resolving', error: net('Uncategorized') })).toBe('dns');
    expect(kind({ step: 'Resolving', error: { code: 'Timeout' } })).toBe('dns_timeout');
    expect(kind({ step: 'Connecting', error: net('ConnectionRefused') })).toBe('refused');
    expect(kind({ step: 'Connecting', error: net('HostUnreachable') })).toBe('unreachable');
    expect(kind({ step: 'Connecting', error: { code: 'Timeout' } })).toBe('tcp_timeout');
    expect(kind({ step: 'Connecting', error: net('Other') })).toBe('tcp_other');
    expect(kind({ step: 'Handshake', error: { code: 'Timeout' } })).toBe('hs_timeout');
    expect(kind({ step: 'Handshake', error: net('ConnectionReset') })).toBe('hs_dropped');
    expect(kind({ step: 'Handshake', error: net('Kex') })).toBe('hs_algo');
    expect(kind({ step: 'Handshake', error: net('protocol error') })).toBe('hs_protocol');
    expect(kind({ step: 'OpeningChannel', error: net('disconnected') })).toBe('channel');
  });

  it('blames the jump host for a hop after the first', () => {
    expect(kind({ step: 'Connecting', error: net('disconnected'), viaJump: true })).toBe('jump');
  });

  it('reports the accepted methods for a refused login', () => {
    const d = diagnose({
      ...base,
      step: 'Authenticating',
      error: { code: 'AuthFailed', detail: { methods_left: ['publickey', 'password'] } }
    });
    expect(d.kind).toBe('auth');
    expect(d.methods).toEqual(['publickey', 'password']);
  });

  it('explains failures before any step and cancellations', () => {
    expect(kind({ step: null, error: { code: 'Validation', detail: 'username required' } })).toBe(
      'no_user'
    );
    expect(kind({ step: null, error: { code: 'VaultLocked' } })).toBe('prepare');
    expect(kind({ step: 'Handshake', error: { code: 'Cancelled' } })).toBe('cancelled');
  });

  it('quotes the last tried address', () => {
    const d = diagnose({
      step: 'Connecting',
      error: net('ConnectionRefused'),
      viaJump: false,
      attempts: [
        { addr: '[2001:db8::1]:22', failure: 'Timeout' },
        { addr: '203.0.113.7:22', failure: 'ConnectionRefused' }
      ]
    });
    expect(d.addr).toBe('203.0.113.7:22');
  });
});
