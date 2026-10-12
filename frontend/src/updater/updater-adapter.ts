import { isTauri } from '@tauri-apps/api/core';
import type { PendingUpdate, UpdateDownloadEvent, UpdaterAdapter, UpdateErrorCode } from './updater-types';

export class UpdaterAdapterError extends Error {
  constructor(readonly code: UpdateErrorCode) {
    super(code);
    this.name = 'UpdaterAdapterError';
  }
}

export function createNativeUpdaterAdapter(): UpdaterAdapter {
  return {
    kind: 'native',
    async check() {
      if (!isTauri()) throw new UpdaterAdapterError('unsupported');
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (!update) return null;
      return {
        currentVersion: update.currentVersion,
        version: update.version,
        body: update.body,
        date: update.date,
        download: onEvent => update.download(event => onEvent(event as UpdateDownloadEvent)),
        install: () => update.install(),
        close: () => update.close(),
      } satisfies PendingUpdate;
    },
    async relaunch() {
      if (!isTauri()) throw new UpdaterAdapterError('unsupported');
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    },
  };
}

export function createUnsupportedUpdaterAdapter(): UpdaterAdapter {
  return {
    kind: 'unsupported',
    async check() { throw new UpdaterAdapterError('unsupported'); },
    async relaunch() { throw new UpdaterAdapterError('unsupported'); },
  };
}

export type MockUpdaterScenario = 'up-to-date' | 'available' | 'network-error' | 'invalid-index' | 'unsupported';

export interface MockUpdaterCalls {
  check: number;
  download: number;
  install: number;
  close: number;
  relaunch: number;
}

export function createMockUpdaterAdapter(scenario: MockUpdaterScenario = 'up-to-date') {
  const calls: MockUpdaterCalls = { check: 0, download: 0, install: 0, close: 0, relaunch: 0 };
  const adapter: UpdaterAdapter = {
    kind: 'mock',
    async check() {
      calls.check++;
      if (scenario === 'network-error') throw new UpdaterAdapterError('network');
      if (scenario === 'invalid-index') throw new UpdaterAdapterError('metadata');
      if (scenario === 'unsupported') throw new UpdaterAdapterError('unsupported');
      if (scenario === 'up-to-date') return null;
      return {
        currentVersion: '0.1.1-rc.7',
        version: '0.1.1-rc.8',
        body: '修复问题并改进应用内更新流程。',
        date: '2026-10-12T10:00:00Z',
        async download(onEvent) {
          calls.download++;
          onEvent({ event: 'Started', data: { contentLength: 1024 * 1024 } });
          onEvent({ event: 'Progress', data: { chunkLength: 512 * 1024 } });
          onEvent({ event: 'Progress', data: { chunkLength: 512 * 1024 } });
          onEvent({ event: 'Finished' });
        },
        async install() { calls.install++; },
        async close() { calls.close++; },
      } satisfies PendingUpdate;
    },
    async relaunch() { calls.relaunch++; },
  };
  return { adapter, calls };
}
