import { Channel, isTauri } from '@tauri-apps/api/core';
import type { IpcClient } from '../ipc/client';
import type { UpdateChannel } from '../../../contracts/v1/UpdateChannel';
import type {
  PendingUpdate,
  UpdateCheckResponse,
  UpdateDownloadEvent,
  UpdateErrorCode,
  UpdaterAdapter,
  UpdaterCheckResult,
} from './updater-types';

export class UpdaterAdapterError extends Error {
  constructor(readonly code: UpdateErrorCode) {
    super(code);
    this.name = 'UpdaterAdapterError';
  }
}

function checkResultToOutcome(response: UpdateCheckResponse, client: IpcClient, expectedChannel: UpdateChannel): UpdaterCheckResult {
  if ('channel' in response && response.channel !== expectedChannel) return { kind: 'failed', code: 'metadata' };
  switch (response.kind) {
    case 'notConfigured': return { kind: 'notConfigured' };
    case 'channelEmpty': return { kind: 'channelEmpty' };
    case 'upToDate': return { kind: 'upToDate', installedVersion: response.installedVersion };
    case 'blocked':
      return {
        kind: 'blocked',
        installedVersion: response.installedVersion,
        targetVersion: response.targetVersion,
        reason: response.reason,
      };
    case 'failed': return { kind: 'failed', code: response.code };
    case 'available': {
      const update: PendingUpdate = {
        currentVersion: response.installedVersion,
        version: response.targetVersion,
        body: response.releaseNotes,
        date: response.publishedAt,
        async download(onEvent) {
          const output = new Channel<UpdateDownloadEvent>();
          output.onmessage = onEvent;
          const result = await client.call('updater_download', {}, output);
          if (result.kind === 'failed') throw new UpdaterAdapterError(result.code);
        },
        async install() {
          const result = await client.call('updater_install', {});
          if (result.kind === 'failed') throw new UpdaterAdapterError(result.code);
        },
        async close() { await client.call('updater_cancel', {}); },
      };
      return { kind: 'available', update };
    }
  }
}

export function createNativeUpdaterAdapter(client: IpcClient): UpdaterAdapter {
  return {
    kind: 'native',
    async check(_channel: UpdateChannel) {
      if (!isTauri()) return { kind: 'failed', code: 'unsupported' };
      try {
        return checkResultToOutcome(await client.call('updater_check', {}), client, _channel);
      } catch {
        return { kind: 'failed', code: 'unknown' };
      }
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
    async check() { return { kind: 'failed', code: 'unsupported' }; },
    async relaunch() { throw new UpdaterAdapterError('unsupported'); },
  };
}

export type MockUpdaterScenario =
  | 'up-to-date'
  | 'available'
  | 'stable-to-preview-upgrade'
  | 'downgrade'
  | 'channel-empty'
  | 'network-error'
  | 'invalid-index'
  | 'signature-error'
  | 'unsupported';

export interface MockUpdaterCalls {
  check: number;
  download: number;
  install: number;
  close: number;
  relaunch: number;
}

function mockUpdate(calls: MockUpdaterCalls, currentVersion: string, version: string, downloadError?: UpdateErrorCode): PendingUpdate {
  return {
    currentVersion,
    version,
    body: '修复问题并改进应用内更新流程。',
    date: '2026-10-12T10:00:00Z',
    async download(onEvent) {
      calls.download++;
      if (downloadError) throw new UpdaterAdapterError(downloadError);
      onEvent({ event: 'Started', data: { contentLength: 1024 * 1024 } });
      onEvent({ event: 'Progress', data: { chunkLength: 512 * 1024 } });
      onEvent({ event: 'Progress', data: { chunkLength: 512 * 1024 } });
      onEvent({ event: 'Finished' });
    },
    async install() { calls.install++; },
    async close() { calls.close++; },
  };
}

export function createMockUpdaterAdapter(scenario: MockUpdaterScenario = 'up-to-date') {
  const calls: MockUpdaterCalls = { check: 0, download: 0, install: 0, close: 0, relaunch: 0 };
  const adapter: UpdaterAdapter = {
    kind: 'mock',
    async check(channel) {
      calls.check++;
      if (scenario === 'network-error') return { kind: 'failed', code: 'network' };
      if (scenario === 'invalid-index') return { kind: 'failed', code: 'metadata' };
      if (scenario === 'unsupported') return { kind: 'failed', code: 'unsupported' };
      if (scenario === 'channel-empty') return { kind: 'channelEmpty' };
      if (scenario === 'up-to-date') return { kind: 'upToDate', installedVersion: '0.1.1-rc.8' };
      if (scenario === 'downgrade') {
        const update = mockUpdate(calls, '0.1.2', channel === 'preview' ? '0.1.2-rc.2' : '0.1.1');
        return { kind: 'available', update };
      }
      if (scenario === 'stable-to-preview-upgrade') {
        const version = channel === 'preview' ? '0.1.2-rc.1' : '0.1.2';
        return { kind: 'available', update: mockUpdate(calls, '0.1.1', version) };
      }
      const currentVersion = '0.1.1-rc.8';
      const version = channel === 'stable' ? '0.1.1' : '0.1.1-rc.9';
      return {
        kind: 'available',
        update: mockUpdate(calls, currentVersion, version, scenario === 'signature-error' ? 'signature' : undefined),
      };
    },
    async relaunch() { calls.relaunch++; },
  };
  return { adapter, calls };
}
