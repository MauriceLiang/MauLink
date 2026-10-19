import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  DEFAULT_INLINE_FILES_PREFERENCE,
  readInlineFilesPreference,
  resolveInlineFilesLayout,
  saveInlineFilesPreference,
  WORKSPACE_LAYOUT_KEY,
} from '../src/stores/workspace-layout';

const values = new Map<string, string>();
const storage = {
  getItem: (key: string) => values.get(key) ?? null,
  setItem: (key: string, value: string) => { values.set(key, value); },
};
beforeEach(() => { values.clear(); vi.stubGlobal('localStorage', storage); });
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });

describe('per-server inline files layout', () => {
  it('uses isolated defaults and ignores malformed stored preferences', () => {
    storage.setItem(WORKSPACE_LAYOUT_KEY, '{not json');
    expect(readInlineFilesPreference('server-a')).toEqual(DEFAULT_INLINE_FILES_PREFERENCE);
    storage.setItem(WORKSPACE_LAYOUT_KEY, JSON.stringify({ version: 1, servers: {
      'server-a': { expanded: 'yes', ratio: 0.3 },
      'server-b': { expanded: true, ratio: 0.3 },
    } }));
    expect(readInlineFilesPreference('server-a')).toEqual(DEFAULT_INLINE_FILES_PREFERENCE);
    expect(readInlineFilesPreference('server-b')).toEqual({ expanded: true, ratio: 0.3 });
  });

  it('stores each server preference and clamps ratios to supported bounds', () => {
    saveInlineFilesPreference('server-a', { expanded: true, ratio: 0.9 });
    saveInlineFilesPreference('server-b', { expanded: false, ratio: 0.1 });
    expect(readInlineFilesPreference('server-a')).toEqual({ expanded: true, ratio: 0.42 });
    expect(readInlineFilesPreference('server-b')).toEqual({ expanded: false, ratio: 0.2 });
    expect(JSON.parse(storage.getItem(WORKSPACE_LAYOUT_KEY)!).version).toBe(1);
  });

  it('reserves terminal and splitter space and uses density-specific minimums', () => {
    expect(resolveInlineFilesLayout(405, 0.3, false).available).toBe(false);
    expect(resolveInlineFilesLayout(394, 0.3, true).available).toBe(true);
    const normal = resolveInlineFilesLayout(800, 0.3, false);
    expect(normal.filesHeight).toBe(240);
    expect(normal.filesHeight).toBeLessThanOrEqual(normal.maxFilesHeight);
    expect(normal.maxFilesHeight).toBeLessThanOrEqual(336);
  });

  it('treats blocked browser storage as an optional preference failure', () => {
    vi.stubGlobal('localStorage', {
      getItem() { throw new DOMException('blocked'); },
      setItem() { throw new DOMException('blocked'); },
    });
    expect(readInlineFilesPreference('server-a')).toEqual(DEFAULT_INLINE_FILES_PREFERENCE);
    expect(() => saveInlineFilesPreference('server-a', { expanded: true, ratio: 0.3 })).not.toThrow();
  });
});
