import { describe, expect, it } from 'vitest';
import { FLAG_CODES } from './codes';
import { ALIASES, flagName, isKnownFlag, normalizeCode, searchFlags } from './flags';

describe('normalizeCode', () => {
  it('resolves aliases to canonical codes', () => {
    expect(normalizeCode('uk')).toBe('gb');
    expect(normalizeCode('usa')).toBe('us');
    expect(normalizeCode('holland')).toBe('nl');
    expect(normalizeCode('england')).toBe('gb-eng');
  });

  it('resolves Russian aliases, case-insensitively and with ё folded', () => {
    expect(normalizeCode('РФ')).toBe('ru');
    expect(normalizeCode('  Германия ')).toBe('de');
    expect(normalizeCode('Нидерланды')).toBe('nl');
  });

  it('accepts canonical codes in any case', () => {
    expect(normalizeCode('RU')).toBe('ru');
    expect(normalizeCode('gb-eng')).toBe('gb-eng');
  });

  it('returns null for unknown, empty and missing input', () => {
    expect(normalizeCode('zz')).toBeNull();
    expect(normalizeCode('')).toBeNull();
    expect(normalizeCode('   ')).toBeNull();
    expect(normalizeCode(null)).toBeNull();
    expect(normalizeCode(undefined)).toBeNull();
  });

  it('keeps every alias target a bundled code', () => {
    for (const target of Object.values(ALIASES)) {
      expect(isKnownFlag(target)).toBe(true);
    }
  });
});

describe('flagName', () => {
  it('returns localized names from Intl.DisplayNames', () => {
    expect(flagName('de', 'en')).toBe('Germany');
    expect(flagName('de', 'ru')).toBe('Германия');
    expect(flagName('ru', 'en')).toBe('Russia');
  });

  it('resolves aliases before naming', () => {
    expect(flagName('uk', 'en')).toBe('United Kingdom');
    expect(flagName('holland', 'ru')).toBe('Нидерланды');
  });

  it('names UK subdivisions with the hand-written table', () => {
    expect(flagName('gb-eng', 'ru')).toBe('Англия');
    expect(flagName('gb-sct', 'en')).toBe('Scotland');
  });

  it('gives a non-empty name for every bundled code in both locales', () => {
    for (const code of FLAG_CODES) {
      expect(flagName(code, 'en').length).toBeGreaterThan(0);
      expect(flagName(code, 'ru').length).toBeGreaterThan(0);
    }
  });

  it('falls back to the upper-cased code for unknown input', () => {
    expect(flagName('zzz', 'en')).toBe('ZZZ');
  });
});

describe('searchFlags', () => {
  it('finds a country by Russian name prefix', () => {
    expect(searchFlags('герм', [])[0]).toBe('de');
    expect(searchFlags('япон', [])).toContain('jp');
  });

  it('finds a country by English name prefix', () => {
    expect(searchFlags('germ', [])[0]).toBe('de');
    expect(searchFlags('japan', [])[0]).toBe('jp');
  });

  it('finds a country by substring in either language', () => {
    expect(searchFlags('штат', [])).toContain('us');
    expect(searchFlags('kingdom', [])).toContain('gb');
  });

  it('ranks exact code and alias matches first', () => {
    expect(searchFlags('ru', [])[0]).toBe('ru');
    expect(searchFlags('uk', [])[0]).toBe('gb');
    expect(searchFlags('usa', [])[0]).toBe('us');
    expect(searchFlags('holl', [])[0]).toBe('nl');
  });

  it('finds Russian aliases typed in Cyrillic', () => {
    expect(searchFlags('рф', [])[0]).toBe('ru');
  });

  it('returns nothing for a query that matches no flag', () => {
    expect(searchFlags('qqqqzz', [])).toEqual([]);
  });

  it('returns every code for an empty query', () => {
    expect(searchFlags('', [])).toHaveLength(FLAG_CODES.length);
  });

  it('puts recent codes first among equal matches, in recent order', () => {
    const withRecent = searchFlags('', ['fr', 'de']);
    expect(withRecent.slice(0, 2)).toEqual(['fr', 'de']);
  });

  it('ignores recent entries that are not known flags', () => {
    expect(searchFlags('', ['zz', 'de'])[0]).toBe('de');
  });
});
