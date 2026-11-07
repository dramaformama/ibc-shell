/** Mock: DNS answers and an offline stand-in for the ipapi.is location API (ADR-011). */
import type { Commands } from '../types';
import { db, delay, notFound, ok } from './state';

/** Seed host label prefix → a public address in that city's data-centre range. */
const IPS: Record<string, string> = {
  'fra-db': '88.99.10.5',
  fra: '5.75.130.10',
  ams: '31.3.96.20',
  hel: '65.21.4.30',
  'home-nas': '73.15.2.40'
};

/** Public address → [country code, city, lat, lon]. */
const PLACES: Record<string, [string, string, number, number]> = {
  '5.75.130.10': ['DE', 'Frankfurt am Main', 50.11, 8.68],
  '88.99.10.5': ['DE', 'Falkenstein', 50.48, 12.37],
  '31.3.96.20': ['NL', 'Amsterdam', 52.37, 4.9],
  '65.21.4.30': ['FI', 'Helsinki', 60.17, 24.94],
  '73.15.2.40': ['US', 'San Jose', 37.34, -121.89]
};

export const hostLookupIp: Commands['hostLookupIp'] = async (id) => {
  await delay(60);
  const host = db.hosts.find((h) => h.id === id);
  if (!host) return notFound();
  const key = Object.keys(IPS).find((k) => host.label.startsWith(k));
  return ok(key ? [IPS[key]!] : [host.hostname]);
};

/** Answers like `https://api.ipapi.is/?q=<ip>`. */
export const geoFetch: typeof fetch = async (input) => {
  await delay(150);
  const ip = new URL(String(input)).searchParams.get('q') ?? '';
  const p = PLACES[ip];
  const body = p
    ? { ip, location: { country_code: p[0], city: p[1], latitude: p[2], longitude: p[3] } }
    : { ip, error: 'not found' };
  return new Response(JSON.stringify(body), { headers: { 'content-type': 'application/json' } });
};
