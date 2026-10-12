import { describe, expect, it, vi } from 'vitest';
import { createMockUpdaterAdapter, UpdaterAdapterError } from '../src/updater/updater-adapter';
import { createUpdaterController } from '../src/updater/updater-controller';
import type { PendingUpdate, UpdaterAdapter } from '../src/updater/updater-types';

function makeUpdate(overrides: Partial<PendingUpdate> = {}): PendingUpdate {
  return {
    currentVersion: '0.1.1-rc.7',
    version: '0.1.1-rc.8',
    body: 'Release notes',
    async download() {},
    async install() {},
    async close() {},
    ...overrides,
  };
}

function makeAdapter(check: () => Promise<PendingUpdate | null>): UpdaterAdapter {
  return { kind: 'mock', check, async relaunch() {} };
}

describe('UpdaterController', () => {
  it('shows up to date without closing a missing update resource', async () => {
    const adapter = createMockUpdaterAdapter('up-to-date');
    const controller = createUpdaterController(adapter.adapter, () => '0.1.1-rc.7');

    await controller.checkForUpdates();

    expect(controller.state.value).toMatchObject({ phase: 'upToDate', currentVersion: '0.1.1-rc.7' });
    expect(adapter.calls).toMatchObject({ check: 1, close: 0, download: 0, install: 0 });
  });

  it('does not download or install as a side effect of finding an update', async () => {
    const adapter = createMockUpdaterAdapter('available');
    const controller = createUpdaterController(adapter.adapter);

    await controller.checkForUpdates();

    expect(controller.state.value).toMatchObject({ phase: 'available', latestVersion: '0.1.1-rc.8', releaseNotes: '修复问题并改进应用内更新流程。' });
    expect(adapter.calls).toMatchObject({ download: 0, install: 0, close: 0 });
  });

  it('accumulates byte chunks and keeps an unknown content length indeterminate', async () => {
    const update = makeUpdate({
      download: async onEvent => {
        onEvent({ event: 'Started', data: {} });
        onEvent({ event: 'Progress', data: { chunkLength: 2048 } });
        onEvent({ event: 'Progress', data: { chunkLength: 4096 } });
        onEvent({ event: 'Finished' });
      },
    });
    const controller = createUpdaterController(makeAdapter(async () => update));
    await controller.checkForUpdates();
    await controller.downloadUpdate();

    expect(controller.state.value).toMatchObject({ phase: 'downloaded', downloadedBytes: 6144 });
    expect(controller.state.value.totalBytes).toBeUndefined();
  });

  it('closes an abandoned update and requires another check before replacing it', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ close });
    const check = vi.fn(async () => update);
    const controller = createUpdaterController(makeAdapter(check));
    await controller.checkForUpdates();
    await controller.checkForUpdates();
    expect(check).toHaveBeenCalledTimes(1);
    expect(close).not.toHaveBeenCalled();

    await controller.dismiss();
    expect(close).toHaveBeenCalledTimes(1);
    expect(controller.state.value.phase).toBe('idle');
  });

  it('serializes concurrent checks', async () => {
    let finish!: (value: null) => void;
    const check = vi.fn(() => new Promise<null>(resolve => { finish = resolve; }));
    const controller = createUpdaterController(makeAdapter(check));
    const first = controller.checkForUpdates();
    const second = controller.checkForUpdates();
    await Promise.resolve();
    expect(check).toHaveBeenCalledTimes(1);
    finish(null);
    await Promise.all([first, second]);
    expect(controller.state.value.phase).toBe('upToDate');
  });

  it('maps metadata, signature and unsupported failures into stable states', async () => {
    const invalid = createUpdaterController(makeAdapter(async () => { throw new UpdaterAdapterError('metadata'); }));
    const signature = createUpdaterController(makeAdapter(async () => { throw new UpdaterAdapterError('signature'); }));
    const unsupported = createUpdaterController(createMockUpdaterAdapter('unsupported').adapter);
    await invalid.checkForUpdates();
    await signature.checkForUpdates();
    await unsupported.checkForUpdates();

    expect(invalid.state.value).toMatchObject({ phase: 'error', errorCode: 'metadata' });
    expect(signature.state.value).toMatchObject({ phase: 'error', errorCode: 'signature' });
    expect(unsupported.state.value).toMatchObject({ phase: 'unsupported', errorCode: 'unsupported' });
  });

  it('does not close the update resource when the user asks to install later', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ close });
    const controller = createUpdaterController(makeAdapter(async () => update));
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();
    controller.cancelInstallRequest();

    expect(controller.state.value.phase).toBe('downloaded');
    expect(close).not.toHaveBeenCalled();
  });

  it('installs only after the explicit request and relaunches macOS after install', async () => {
    const install = vi.fn(async () => {});
    const relaunch = vi.fn(async () => {});
    const update = makeUpdate({ install });
    const controller = createUpdaterController({ ...makeAdapter(async () => update), relaunch });
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    expect(install).not.toHaveBeenCalled();
    controller.requestInstall();
    await controller.installConfirmed('macos');

    expect(install).toHaveBeenCalledTimes(1);
    expect(relaunch).toHaveBeenCalledTimes(1);
    expect(controller.state.value.phase).toBe('installing');
  });

  it('hands Windows installation to the updater installer without an extra relaunch', async () => {
    const install = vi.fn(async () => {});
    const relaunch = vi.fn(async () => {});
    const update = makeUpdate({ install });
    const controller = createUpdaterController({ ...makeAdapter(async () => update), relaunch });
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
    const controller = createUpdaterController(makeAdapter(async () => update));
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    controller.requestInstall();
    await controller.installConfirmed('linux');

    expect(install).not.toHaveBeenCalled();
    expect(controller.state.value).toMatchObject({ phase: 'unsupported', errorCode: 'unsupported' });
  });

  it('closes a downloaded updater resource when its owner is disposed', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ close });
    const controller = createUpdaterController(makeAdapter(async () => update));
    await controller.checkForUpdates();
    await controller.downloadUpdate();
    await controller.dispose();

    expect(close).toHaveBeenCalledOnce();
  });

  it('releases resources after a download failure so the user can retry', async () => {
    const close = vi.fn(async () => {});
    const update = makeUpdate({ download: async () => { throw new Error('network unavailable'); }, close });
    const controller = createUpdaterController(makeAdapter(async () => update));
    await controller.checkForUpdates();
    await controller.downloadUpdate();

    expect(controller.state.value).toMatchObject({ phase: 'error', errorCode: 'network' });
    expect(close).toHaveBeenCalledTimes(1);
  });
});
