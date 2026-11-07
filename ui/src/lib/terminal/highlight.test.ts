import { describe, expect, it } from 'vitest';
import { Highlighter } from './highlight';

const C = (code: number, s: string) => `\x1b[${code}m${s}\x1b[39m`;

describe('Highlighter', () => {
  it('colours plain output', () => {
    const h = new Highlighter();
    expect(h.feed('IPv4 address for ens3: 138.124.38.121\r\n')).toBe(
      `IPv4 address for ens3: ${C(36, '138.124.38.121')}\r\n`
    );
    expect(h.feed('Memory usage: 17%')).toBe(`Memory usage: ${C(35, '17%')}`);
    expect(h.feed('Permission denied')).toBe(`Permission ${C(31, 'denied')}`);
    expect(h.feed('see https://ubuntu.com/pro.')).toBe(`see ${C(34, 'https://ubuntu.com/pro')}.`);
  });

  it('leaves coloured and full-screen output alone', () => {
    const h = new Highlighter();
    expect(h.feed('\x1b[32mok 1.2.3.4\x1b[0m done')).toBe(
      `\x1b[32mok 1.2.3.4\x1b[0m ${C(32, 'done')}`
    );
    expect(h.feed('\x1b[?1049herror\x1b[?1049l')).toBe('\x1b[?1049herror\x1b[?1049l');
  });

  it('keeps escape sequences split across chunks intact', () => {
    const h = new Highlighter();
    expect(h.feed('a\x1b[3')).toBe('a');
    expect(h.feed('2mx\x1b[0m')).toBe('\x1b[32mx\x1b[0m');
    expect(h.feed('\x1b]0;title')).toBe('');
    expect(h.feed('\x07ok')).toBe(`\x1b]0;title\x07${C(32, 'ok')}`);
  });
});
