import { describe, expect, it } from 'vitest';
import { cwdFromTitle, expandHome, parseOsc7, shellQuote } from './cwd';

describe('terminal folder', () => {
  it('reads OSC 7 file URLs', () => {
    expect(parseOsc7('file://web-1/var/www/my%20site')).toBe('/var/www/my site');
    expect(parseOsc7('file:///home/me')).toBe('/home/me');
    expect(parseOsc7('http://x/y')).toBeNull();
    expect(parseOsc7('file://host/%E0%A4%A')).toBeNull();
    expect(parseOsc7('file://host/a%0Ab')).toBeNull();
  });

  it('reads the Debian-style title', () => {
    expect(cwdFromTitle('root@web-1: /etc/nginx')).toBe('/etc/nginx');
    expect(cwdFromTitle('deploy@app.example.com: ~/app')).toBe('~/app');
    expect(cwdFromTitle('me@mac:~')).toBe('~');
    expect(cwdFromTitle('vim notes.txt')).toBeNull();
    expect(cwdFromTitle('root@web-1: relative')).toBeNull();
  });

  it('expands the home folder', () => {
    expect(expandHome('~', '/root')).toBe('/root');
    expect(expandHome('~/app', '/home/deploy/')).toBe('/home/deploy/app');
    expect(expandHome('/etc', '/root')).toBe('/etc');
  });

  it('quotes paths for the shell', () => {
    expect(shellQuote('/Users/me/a.txt')).toBe('/Users/me/a.txt');
    expect(shellQuote("/Users/me/it's here.png")).toBe(`'/Users/me/it'\\''s here.png'`);
  });
});
