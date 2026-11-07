import { describe, expect, it } from 'vitest';
import {
  binaryBytes,
  needsPasteConfirm,
  pasteLineCount,
  safeLink,
  sanitizePaste,
  sanitizeTitle,
  utf8
} from './text';

describe('terminal text helpers', () => {
  it('strips ESC from paste but keeps newlines and tabs', () => {
    expect(sanitizePaste('a\x1b[201~b\tc\nd')).toBe('a[201~b\tc\nd');
  });
  it('counts lines and confirms multi-line paste only', () => {
    expect(pasteLineCount('ls\n')).toBe(1);
    expect(needsPasteConfirm('ls\n')).toBe(false);
    expect(needsPasteConfirm('a\r\nb')).toBe(true);
  });
  it('sanitizes titles and links', () => {
    expect(sanitizeTitle('a\x07b‮c')).toBe('abc');
    expect(safeLink('javascript:alert(1)')).toBeNull();
    expect(safeLink('https://example.com')).toBe('https://example.com/');
  });
  it('encodes bytes', () => {
    expect([...utf8('é')]).toEqual([0xc3, 0xa9]);
    expect([...binaryBytes('\xff\x01')]).toEqual([255, 1]);
  });
});
