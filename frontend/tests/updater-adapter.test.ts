import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createNativeUpdaterAdapter } from '../src/updater/updater-adapter';

const { check } = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check }));

describe('native updater configuration', () => {
  beforeEach(() => { check.mockReset(); });

  it.each(['Command check not found', new Error('Command check not found')])(
    'reports an unconfigured build when the updater plugin is absent',
    async error => {
      check.mockRejectedValue(error);
      await expect(createNativeUpdaterAdapter().check()).rejects.toMatchObject({ code: 'unsupported' });
    },
  );

  it('preserves errors from a configured updater', async () => {
    const error = new Error('network unavailable');
    check.mockRejectedValue(error);
    await expect(createNativeUpdaterAdapter().check()).rejects.toBe(error);
  });

  it('keeps the no-update result from a configured updater', async () => {
    check.mockResolvedValue(null);
    await expect(createNativeUpdaterAdapter().check()).resolves.toBeNull();
  });
});
