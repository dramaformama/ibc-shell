import { describe, expect, it } from 'vitest';
import { isLoopback, route } from './forwards.svelte';

const base = { bind_addr: '127.0.0.1', bind_port: 8080, dest_host: 'db', dest_port: 5432 };

describe('forward summary', () => {
  it('shows local and remote routes', () => {
    expect(route({ ...base, kind: 'local' })).toBe('localhost:8080 → db:5432');
    expect(route({ ...base, kind: 'remote', bind_addr: '0.0.0.0' })).toBe('0.0.0.0:8080 → db:5432');
  });
  it('prefers the bound port and marks auto ports', () => {
    expect(route({ ...base, kind: 'local', bind_port: 0 })).toBe('localhost:auto → db:5432');
    expect(route({ ...base, kind: 'local', bind_port: 0 }, 50123)).toBe(
      'localhost:50123 → db:5432'
    );
  });
  it('shows SOCKS without a destination', () => {
    expect(route({ ...base, kind: 'dynamic', dest_host: null, dest_port: null }, 1080)).toBe(
      'localhost:1080 · SOCKS5'
    );
  });
  it('knows which addresses stay on this computer', () => {
    for (const a of ['127.0.0.1', 'localhost', '::1', ' LOCALHOST '])
      expect(isLoopback(a)).toBe(true);
    for (const a of ['0.0.0.0', '192.168.1.5', '::']) expect(isLoopback(a)).toBe(false);
  });
});
