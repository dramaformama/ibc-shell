import { describe, expect, it } from 'vitest';
import { certTone, daysLeft, portChips } from './ports';

describe('portChips', () => {
  it('merges IPv4/IPv6 duplicates and marks loopback', () => {
    expect(portChips('0.0.0.0:443->443/tcp, :::443->443/tcp, 127.0.0.1:5432->5432/tcp')).toEqual([
      { text: '443', proto: 'tcp', local: false, internal: false },
      { text: '5432', proto: 'tcp', local: true, internal: false }
    ]);
  });

  it('maps differing ports and exposed-only ones', () => {
    expect(portChips('0.0.0.0:8080->80/tcp, 9000/udp')).toEqual([
      { text: '8080→80', proto: 'tcp', local: false, internal: false },
      { text: '9000', proto: 'udp', local: false, internal: true }
    ]);
    expect(portChips('')).toEqual([]);
  });
});

describe('certificates', () => {
  it('counts days and tones', () => {
    const now = Date.UTC(2026, 10, 7);
    expect(daysLeft(now + 6.5 * 86_400_000, now)).toBe(6);
    expect(daysLeft(now - 1000, now)).toBe(-1);
    expect(certTone(6)).toBe('bad');
    expect(certTone(20)).toBe('warn');
    expect(certTone(60)).toBe('ok');
  });
});
