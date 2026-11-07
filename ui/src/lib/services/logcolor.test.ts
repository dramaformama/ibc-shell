import { describe, expect, it } from 'vitest';
import { colorLine, shortTime, stripAnsi } from './logcolor';

const joined = (s: { text: string }[]) => s.map((x) => x.text).join('');

describe('colorLine', () => {
  it('keeps the container colours', () => {
    const line = '\x1b[32mok\x1b[0m done \x1b[1;31mfail\x1b[0m';
    const segs = colorLine(line).segs;
    expect(joined(segs)).toBe('ok done fail');
    expect(segs[0]).toEqual({ text: 'ok', cls: 'a2' });
    expect(segs.at(-1)).toEqual({ text: 'fail', cls: 'a1 b' });
    expect(stripAnsi(line)).toBe('ok done fail');
  });

  it('highlights plain lines without changing the text', () => {
    const line =
      '2026-11-07T10:00:00.123Z 91.204.17.6 - "GET /api HTTP/1.1" 502 12 level=ERROR user=bob';
    const { ts, segs } = colorLine(line);
    expect(ts).toBe('2026-11-07T10:00:00.123Z');
    expect(joined(segs)).toBe(line.slice(ts!.length + 1));
    expect(segs.find((s) => s.text === '91.204.17.6')?.cls).toBe('ip');
    expect(segs.find((s) => s.text === 'GET')?.cls).toBe('meth');
    expect(segs.find((s) => s.text === '502')?.cls).toBe('err b');
    expect(segs.find((s) => s.text === 'ERROR')?.cls).toBe('err b');
    expect(segs.find((s) => s.text === 'user')?.cls).toBe('key');
  });

  it('leaves ordinary text alone', () => {
    expect(colorLine('hello world')).toEqual({
      ts: null,
      segs: [{ text: 'hello world', cls: '' }]
    });
    expect(colorLine('').segs).toEqual([]);
  });

  it('separates the Docker timestamp from coloured output', () => {
    const { ts, segs } = colorLine('2026-10-31T21:05:05.677917327Z \x1b[32mLOG\x1b[0m up');
    expect(ts).toBe('2026-10-31T21:05:05.677917327Z');
    expect(segs[0]).toEqual({ text: 'LOG', cls: 'a2' });
    expect(shortTime('2026-10-15T12:05:05Z', 'en')).toMatch(/^1[45] Oct \d\d:05:05$/);
    expect(shortTime('2026-10-15T12:05:05Z', 'ru')).toMatch(/^1[45] окт \d\d:05:05$/);
  });
});
