import { describe, expect, it } from 'vitest';
import { checkForUpdate, isNewer, parseVersion, RELEASES_PAGE } from './update';

const reply = (body: unknown, ok = true) =>
  (async () => ({ ok, json: async () => body })) as unknown as typeof fetch;

describe('versions', () => {
  it('parses plain versions only', () => {
    expect(parseVersion('v1.2.3')).toEqual([1, 2, 3]);
    expect(parseVersion('0.1.0-beta.1')).toEqual([0, 1, 0]);
    expect(parseVersion('latest')).toBeNull();
  });
  it('compares numerically', () => {
    expect(isNewer('0.1.0', '0.1.1')).toBe(true);
    expect(isNewer('0.9.0', '0.10.0')).toBe(true);
    expect(isNewer('1.0.0', '1.0.0')).toBe(false);
    expect(isNewer('1.0.0', '0.9.9')).toBe(false);
    expect(isNewer('1.0.0', 'junk')).toBe(false);
  });
});

describe('checkForUpdate', () => {
  const url = `${RELEASES_PAGE}/tag/v0.2.0`;
  it('returns a newer release', async () => {
    const r = await checkForUpdate('0.1.0', reply({ tag_name: 'v0.2.0', html_url: url }));
    expect(r).toEqual({ version: '0.2.0', url });
  });
  it('is null when current, prerelease, failed or malformed', async () => {
    expect(await checkForUpdate('0.2.0', reply({ tag_name: 'v0.2.0', html_url: url }))).toBeNull();
    expect(
      await checkForUpdate('0.1.0', reply({ tag_name: 'v0.3.0', prerelease: true }))
    ).toBeNull();
    expect(await checkForUpdate('0.1.0', reply({}, false))).toBeNull();
    expect(await checkForUpdate('0.1.0', reply('x'))).toBeNull();
    expect(
      await checkForUpdate('0.1.0', (async () => Promise.reject(new Error('off'))) as never)
    ).toBeNull();
  });
  it('never trusts a foreign link', async () => {
    const r = await checkForUpdate(
      '0.1.0',
      reply({ tag_name: 'v0.2.0', html_url: 'https://evil.example/x' })
    );
    expect(r?.url).toBe(RELEASES_PAGE);
  });
});
