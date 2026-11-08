import { describe, expect, it } from 'vitest';
import { fitView, project, projectLonLat, spread, WORLD } from './geo';
import { isPublicIp, parseAnswer, parseIpinfo } from './geoip.svelte';
import { GRID } from './world';

describe('map geometry', () => {
  it('projects known countries inside the grid and rejects unknown codes', () => {
    const de = project('DE')!;
    expect(de[0]).toBeGreaterThan(GRID.cols / 2);
    expect(de[1]).toBeGreaterThan(0);
    expect(de[1]).toBeLessThan(GRID.rows / 2);
    expect(project('zz')).toBeNull();
  });

  it('zooms to a European cluster but keeps the world when servers span it', () => {
    const eu = fitView([project('de')!, project('nl')!, project('fi')!]);
    expect(eu.w).toBeLessThan(GRID.cols / 2);
    expect(eu.h / eu.w).toBeCloseTo(GRID.rows / GRID.cols);
    const all = fitView([project('us')!, project('au')!, project('fi')!]);
    expect(all.w).toBeGreaterThan(GRID.cols * 0.8);
    expect(all.x).toBeGreaterThanOrEqual(0);
    expect(all.x + all.w).toBeLessThanOrEqual(GRID.cols + 1e-9);
    expect(fitView([])).toEqual(WORLD);
  });

  it('separates overlapping pins and leaves distant ones alone', () => {
    const [a, b, c] = spread(
      [
        [100, 100],
        [104, 102],
        [400, 300]
      ],
      30
    );
    expect(a).toEqual([100, 100]);
    expect(Math.hypot(a![0] - b![0], a![1] - b![1])).toBeGreaterThanOrEqual(30);
    expect(c).toEqual([400, 300]);
  });

  it('places a city by coordinates', () => {
    const [x, y] = projectLonLat(8.68, 50.11);
    const de = project('de')!;
    expect(Math.abs(x - de[0])).toBeLessThan(3);
    expect(Math.abs(y - de[1])).toBeLessThan(3);
  });
});

describe('ip location', () => {
  it('never sends private or reserved addresses', () => {
    for (const ip of [
      '10.0.0.5',
      '192.168.1.50',
      '172.20.1.1',
      '127.0.0.1',
      '100.64.1.1',
      '169.254.0.1',
      '203.0.113.10',
      '::1',
      'fd00::1',
      'fe80::1',
      'nas.local'
    ])
      expect(isPublicIp(ip), ip).toBe(false);
    for (const ip of ['5.75.130.10', '1.1.1.1', '2a01:4f8::1'])
      expect(isPublicIp(ip), ip).toBe(true);
  });

  it('reads an ipapi.is answer and rejects junk', () => {
    const hit = parseAnswer(
      'a.example',
      '1.1.1.1',
      { location: { country_code: 'DE', city: 'Berlin', latitude: 52.5, longitude: 13.4 } },
      5
    );
    expect(hit).toEqual({
      addr: 'a.example',
      ip: '1.1.1.1',
      cc: 'de',
      city: 'Berlin',
      lat: 52.5,
      lon: 13.4,
      at: 5
    });
    expect(parseAnswer('a', '1.1.1.1', { error: 'x' }, 5)).toBeNull();
    expect(
      parseAnswer(
        'a',
        '1.1.1.1',
        { location: { country_code: '<b>', latitude: 1, longitude: 2 } },
        5
      )
    ).toBeNull();
  });

  it('reads the flat ipapi.is form and an ipinfo.io answer', () => {
    expect(
      parseAnswer('a', '1.1.1.1', { city: 'Moscow', lat: 55.75, lon: 37.6, country_code: 'RU' }, 1)
    ).toMatchObject({ cc: 'ru', city: 'Moscow', lat: 55.75, lon: 37.6 });
    expect(
      parseIpinfo('a', '1.1.1.1', { loc: '55.0415,82.9346', country: 'RU', city: 'Novosibirsk' }, 1)
    ).toMatchObject({ cc: 'ru', city: 'Novosibirsk', lat: 55.0415, lon: 82.9346 });
    expect(parseIpinfo('a', '1.1.1.1', { loc: 'x,y', country: 'RU' }, 1)).toBeNull();
    expect(parseIpinfo('a', '1.1.1.1', null, 1)).toBeNull();
  });
});
