import { describe, expect, it, vi } from 'vitest';
import { createMockUpdaterAdapter, UpdaterAdapterError } from '../src/updater/updater-adapter';
import { createUpdaterController } from '../src/updater/updater-controller';
import type { PendingUpdate, UpdaterAdapter, UpdaterCheckResult } from '../src/updater/updater-types';

function makeUpdate(overrides: Partial<PendingUpdate> = {}): PendingUpdate {
  return {
    currentVersion: '0.1.1-rc.8',
    version: '0.1.1-rc.9',
    body: 'Release notes',
    async download() {},
    async install() {},
    async close() {},
    ...overrides,
  };
}

function available(update: PendingUpdate): UpdaterCheckResult {
  return { kind: 'available', update };
}

function makeAdapter(check: UpdaterAdapter['check']): UpdaterAdapter {
  return { kind: 'mock', check, async relaunch() {} };
}

describe('UpdaterController', () => {
  it('shows up to date without creating or closing an update resource', async () => {
    const check = vi.fn(async () => ({ kind: 'upToDate', installedVersion: '0.1.1-rc.8' } as const));
    const controller = createUpdaterController(makeAdapter(check), () => '0.1.1-rc.8');
    await controller.checkForUpdates();

    expect(controller.state.value).toMatchObject({ phase: 'upToDate', currentVersion: '0.1.1-rc.8', installedChannel: 'preview' });
    expect(check).toHaveBeenCalledWith('preview');
  });

  it('does not download or install as a side effect of finding an upgrade', async () => {
    const adapter = createMockUpdaterAdapter('available');
    const controller = createUpdaterController(adapter.adapter, () => '0.1.1-rc.8');
    await controller.checkForUpdates();

    expect(controller.state.value).toMatchObject({ phase: 'available', latestVersion: '0.1.1-rc.9', transition: 'upgrade', releaseNotes: '修复问题并改进应用内更新流程。' });
    expect(adapter.calls).toMatchObject({ download: 0, install: 0, close: 0 });
  });

  it('tracks byte chunks and leaves an unknown content length indeterminate', async () => {
    const update = makeUpdate({
      download: async onEvent => {
        onEvent({ event: 'Started', data: {} });
        onEvent({ event: 'Progress', data: { chunkLength: 2048 } });
        onEvent({ event: 'Progress', data: { chunkLength: 4096 } });
        onEvent({ event: 'Finished' });
      },
    });
    const controller = createUpdaterController(makeAdapter(async () => available(update)));
    await controller.checkForUpdates();
    await controller.downloadUpdate();

    expect(controller.state.value).toMatchObject({ phase: 'downloaded', downloadedBytes: 6144 });
    expect(controller.state.value.totalBytes).toBeUndefined();
  });

  it('closes a pending update when the channel changes and checks the newly selected channel', async () => {
    const close = vi.fn(async () => {});
    const check = vi.fn(async (channel) => channel === 'preview'
      ? available(makeUpdate({ close }))
      : ({ kind: 'channelEmpty' } as const));
    const controller = createUpdaterController(makeAdapter(check), () => '0.1.1-rc.8');
    await controller.checkForUpdates();
    expect(await controller.setChannel('stable')).toBe(true);
    expect(close).toHaveBeenCalledOnce();
    expect(controller.state.value).toMatchObject({ phase: 'idle', updateChannel: 'stable' });
    await controller.checkForUpdates();
    expect(check).toHaveBeenLastCalledWith('stable');
    expect(controller.state.value.phase).toBe('channelEmpty');
  });

  it('applies a persisted channel even if releasing the previous candidate reports an error', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const controller = createUpdaterController(makeAdapter(async () => available(makeUpdate({
      close: async () => { throw new Error('close failed'); },
    }))), () => '0.1.1-rc.8');
    await controller.checkForUpdates();

    expect(await controller.setChannel('stable')).toBe(true);
    expect(controller.state.value).toMatchObject({ phase: 'error', updateChannel: 'stable', errorCode: 'unknown' });
    warn.mockRestore();
  });

  it('blocks downgrades before download or install when compatibility is unknown', async () => {
    const adapter = createMockUpdaterAdapter('downgrade');
    const controller = createUpdaterController(adapter.adapter, () => '0.1.2');
    await controller.setChannel('preview');
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();

    expect(controller.state.value).toMatchObject({
      phase: 'blocked',
      currentVersion: '0.1.2',
      latestVersion: '0.1.2-rc.2',
      transition: 'downgrade',
      blockedReason: 'compatibilityUnverified',
    });
    expect(adapter.calls).toMatchObject({ download: 0, install: 0, close: 1 });
  });

  it('treats matching versions as up to date and releases the candidate', async () => {
    const close = vi.fn(async () => {});
    const controller = createUpdaterController(makeAdapter(async () => available(makeUpdate({ version: '0.1.1-rc.8', close }))));
    await controller.checkForUpdates();

    expect(controller.state.value.phase).toBe('upToDate');
    expect(close).toHaveBeenCalledOnce();
  });

  it('serializes concurrent checks', async () => {
    let finish!: (value: UpdaterCheckResult) => void;
    const check = vi.fn(() => new Promise<UpdaterCheckResult>(resolve => { finish = resolve; }));
    const controller = createUpdaterController(makeAdapter(check));
    const first = controller.checkForUpdates();
    const second = controller.checkForUpdates();
    await Promise.resolve();
    expect(check).toHaveBeenCalledTimes(1);
    finish({ kind: 'upToDate', installedVersion: '0.1.1-rc.8' });
    await Promise.all([first, second]);
    expect(controller.state.value.phase).toBe('upToDate');
  });

  it('maps typed network, metadata, signature, and not-configured results to separate states', async () => {
    const network = createUpdaterController(makeAdapter(async () => ({ kind: 'failed', code: 'network' })));
    const invalid = createUpdaterController(makeAdapter(async () => ({ kind: 'failed', code: 'metadata' })));
    const signature = createUpdaterController(makeAdapter(async () => ({ kind: 'failed', code: 'signature' })));
    const unconfigured = createUpdaterController(makeAdapter(async () => ({ kind: 'notConfigured' })));
    await network.checkForUpdates();
    await invalid.checkForUpdates();
    await signature.checkForUpdates();
    await unconfigured.checkForUpdates();

    expect(network.state.value).toMatchObject({ phase: 'error', errorCode: 'network' });
    expect(invalid.state.value).toMatchObject({ phase: 'error', errorCode: 'metadata' });
    expect(signature.state.value).toMatchObject({ phase: 'error', errorCode: 'signature' });
    expect(unconfigured.state.value).toMatchObject({ phase: 'unsupported', errorCode: 'unsupported' });
  });

  it('does not close an update when the user postpones installation', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ close });
    const controller = createUpdaterController(makeAdapter(async () => available(update)));
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();
    controller.cancelInstallRequest();

    expect(controller.state.value.phase).toBe('downloaded');
    expect(close).not.toHaveBeenCalled();
  });

  it('installs only after explicit confirmation and relaunches macOS after install', async () => {
    const install = vi.fn(async () => {});
    const relaunch = vi.fn(async () => {});
    const update = makeUpdate({ install });
    const controller = createUpdaterController({ ...makeAdapter(async () => available(update)), relaunch });
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    expect(install).not.toHaveBeenCalled();
    controller.requestInstall();
    await controller.installConfirmed('macos');

    expect(install).toHaveBeenCalledOnce();
    expect(relaunch).toHaveBeenCalledOnce();
    expect(controller.state.value.phase).toBe('installing');
  });

  it('hands Windows installation to the updater installer without an extra relaunch', async () => {
    const install = vi.fn(async () => {});
    const relaunch = vi.fn(async () => {});
    const update = makeUpdate({ install });
    const controller = createUpdaterController({ ...makeAdapter(async () => available(update)), relaunch });
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();
    await controller.installConfirmed('windows');

    expect(install).toHaveBeenCalledOnce();
    expect(relaunch).not.toHaveBeenCalled();
    expect(controller.state.value.phase).toBe('installing');
  });

  it('rejects unsupported platforms before invoking the installer', async () => {
    const install = vi.fn(async () => {});
    const update = makeUpdate({ install });
    const controller = createUpdaterController(makeAdapter(async () => available(update)));
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();
    await controller.installConfirmed('linux');

    expect(install).not.toHaveBeenCalled();
    expect(controller.state.value).toMatchObject({ phase: 'unsupported', errorCode: 'unsupported' });
  });

  it('closes pending resources when disposed and after failed downloads', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ download: async () => { throw new UpdaterAdapterError('network'); }, close });
    const controller = createUpdaterController(makeAdapter(async () => available(update)));
    await controller.checkForUpdates();
    await controller.downloadUpdate();

    expect(controller.state.value).toMatchObject({ phase: 'error', errorCode: 'network' });
    expect(close).toHaveBeenCalledOnce();

    const secondClose = vi.fn(async () => {});
    const second = createUpdaterController(makeAdapter(async () => available(makeUpdate({ close: secondClose }))));
    await second.checkForUpdates();
    await second.dispose();
    expect(secondClose).toHaveBeenCalledOnce();
  });
});
