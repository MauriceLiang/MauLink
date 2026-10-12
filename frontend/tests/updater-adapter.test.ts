import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createIpcClient, type IpcTransport } from '../src/ipc/client';
import { createNativeUpdaterAdapter } from '../src/updater/updater-adapter';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, Channel: class {} }));

function adapterFor(response: unknown) {
  const transport: IpcTransport = {
    async invoke<T>(command: string): Promise<T> {
      if (command === 'updater_check') return response as T;
      throw new Error('Unexpected command: ' + command);
    },
  };
  return createNativeUpdaterAdapter(createIpcClient(transport));
}

describe('native updater command adapter', () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it('reports an unconfigured build without invoking the plugin updater API', async () => {
    await expect(adapterFor({ kind: 'notConfigured' }).check('preview')).resolves.toEqual({ kind: 'notConfigured' });
  });

  it('preserves typed network, metadata and signature outcomes', async () => {
    await expect(adapterFor({ kind: 'failed', code: 'network' }).check('stable')).resolves.toEqual({ kind: 'failed', code: 'network' });
    await expect(adapterFor({ kind: 'failed', code: 'metadata' }).check('preview')).resolves.toEqual({ kind: 'failed', code: 'metadata' });
    await expect(adapterFor({ kind: 'failed', code: 'signature' }).check('preview')).resolves.toEqual({ kind: 'failed', code: 'signature' });
  });

  it('rejects a response for a different persisted channel', async () => {
    await expect(adapterFor({ kind: 'channelEmpty', channel: 'stable' }).check('preview')).resolves.toEqual({ kind: 'failed', code: 'metadata' });
  });

  it('preserves an explicitly empty channel and current version outcomes', async () => {
    await expect(adapterFor({ kind: 'channelEmpty', channel: 'preview' }).check('preview')).resolves.toEqual({ kind: 'channelEmpty' });
    await expect(adapterFor({ kind: 'upToDate', channel: 'stable', installedVersion: '0.1.1' }).check('stable')).resolves.toEqual({ kind: 'upToDate', installedVersion: '0.1.1' });
  });
});
