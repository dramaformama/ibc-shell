import { describe, expect, it } from 'vitest';
import { eventLabel, eventTone, kindLabel } from './docker-events';

describe('docker events', () => {
  it('colours by what happened', () => {
    expect(eventTone('die')).toBe('danger');
    expect(eventTone('oom')).toBe('danger');
    expect(eventTone('start')).toBe('ok');
    expect(eventTone('stop')).toBe('warn');
    expect(eventTone('tag')).toBe('off');
  });
  it('keeps unknown actions as the daemon wrote them', () => {
    expect(eventLabel('rename')).toBe('rename');
    expect(kindLabel('plugin')).toBe('plugin');
  });
});
