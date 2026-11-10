import { describe, expect, it } from 'vitest';
import { ActivityWatch, MIN_HIDDEN_MS } from './activity';

const PROMPT =
  '\x1b]0;root@web-1: ~\x07\x1b]7;file://web-1/root\x07\x1b[1;32mroot@web-1\x1b[0m:\x1b[1;34m~\x1b[0m$ ';

describe('background command notices', () => {
  it('says nothing while the tab is shown', () => {
    const w = new ActivityWatch();
    expect(w.feed('done\r\n' + PROMPT, 99_999)).toBeNull();
  });

  it('reports a prompt that returns after a long command', () => {
    const w = new ActivityWatch();
    w.setHidden(0);
    expect(w.feed('Unpacking…\r\n', 1_000)).toBeNull();
    expect(w.feed('done\r\n' + PROMPT, MIN_HIDDEN_MS + 1)).toBe('done');
    // Once per hidden stint.
    expect(w.feed('\r\n' + PROMPT, MIN_HIDDEN_MS + 2)).toBeNull();
    w.setHidden(null);
    w.setHidden(100_000);
    expect(w.feed(PROMPT, 100_000 + MIN_HIDDEN_MS)).toBe('done');
  });

  it('ignores a prompt that comes back right away', () => {
    const w = new ActivityWatch();
    w.setHidden(0);
    expect(w.feed(PROMPT, 1_000)).toBeNull();
  });

  it('ignores streaming output that is not a prompt', () => {
    const w = new ActivityWatch();
    w.setHidden(0);
    expect(w.feed('GET /index.html 200\r\nGET /a.css 200\r\n', 60_000)).toBeNull();
    expect(w.feed('progress 45%', 61_000)).toBeNull();
  });

  it('finds prompts split across chunks and other shells', () => {
    const w = new ActivityWatch();
    w.setHidden(0);
    expect(w.feed('me@mac ~ ', 10_000)).toBeNull();
    expect(w.feed('%', 10_001)).toBe('done');
    const z = new ActivityWatch();
    z.setHidden(0);
    expect(z.feed('\x1b[36m~/app\x1b[0m main\r\n❯ ', 10_000)).toBe('done');
  });

  it('reports the bell but not the BEL that ends a title', () => {
    const w = new ActivityWatch();
    w.setHidden(0);
    expect(w.feed('\x1b]0;title\x07', 1)).toBeNull();
    expect(w.feed('ding\x07', 2)).toBe('bell');
  });
});
