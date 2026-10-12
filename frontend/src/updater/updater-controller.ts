import { shallowRef } from 'vue';
import {
  channelForReleaseVersion,
  classifyReleaseTransition,
} from '../../../tools/release-channel.mjs';
import { UpdaterAdapterError } from './updater-adapter';
import type {
  PendingUpdate,
  UpdateDisplayState,
  UpdateErrorCode,
  UpdateTransition,
  UpdateChannel,
  UpdaterAdapter,
  UpdaterController,
} from './updater-types';

type Operation = 'check' | 'download' | 'install';

function classifyError(error: unknown, operation: Operation): UpdateErrorCode {
  if (error instanceof UpdaterAdapterError) return error.code;
  if (operation === 'install') return 'install';
  return 'unknown';
}

function installedChannel(version: string): UpdateChannel {
  try { return channelForReleaseVersion(version); }
  catch { return 'stable'; }
}

const initialState = (currentVersion: string, adapter: UpdaterAdapter): UpdateDisplayState => ({
  phase: adapter.kind === 'unsupported' ? 'unsupported' : 'idle',
  currentVersion,
  installedChannel: installedChannel(currentVersion),
  updateChannel: installedChannel(currentVersion),
  downloadedBytes: 0,
  ...(adapter.kind === 'unsupported' ? { errorCode: 'unsupported' as const } : {}),
});

export function createUpdaterController(adapter: UpdaterAdapter, getCurrentVersion: () => string = () => '—'): UpdaterController {
  const state = shallowRef<UpdateDisplayState>(initialState(getCurrentVersion(), adapter));
  let pendingUpdate: PendingUpdate | null = null;
  let operation: Promise<void> | null = null;
  let disposed = false;

  function updateState(patch: Partial<UpdateDisplayState>) {
    if (!disposed) state.value = { ...state.value, ...patch };
  }
  async function releasePendingUpdate() {
    const update = pendingUpdate;
    pendingUpdate = null;
    if (update) await update.close();
  }
  async function run(action: () => Promise<void>): Promise<void> {
    if (operation) return operation;
    const task = Promise.resolve().then(action);
    operation = task;
    try { await task; }
    finally { if (operation === task) operation = null; }
  }
  function setError(error: unknown, stage: Operation) {
    const errorCode = classifyError(error, stage);
    console.warn(`[updater:${stage}] operation failed (${errorCode})`);
    updateState({ phase: errorCode === 'unsupported' ? 'unsupported' : 'error', errorCode });
  }
  function resetUpdateState(channel: UpdateChannel) {
    updateState({
      phase: adapter.kind === 'unsupported' ? 'unsupported' : 'idle',
      updateChannel: channel,
      currentVersion: getCurrentVersion(),
      installedChannel: installedChannel(getCurrentVersion()),
      latestVersion: undefined,
      releaseNotes: undefined,
      transition: undefined,
      blockedReason: undefined,
      downloadedBytes: 0,
      totalBytes: undefined,
      errorCode: adapter.kind === 'unsupported' ? 'unsupported' : undefined,
    });
  }

  const controller: UpdaterController = {
    state,
    adapterKind: adapter.kind,
    setCurrentVersion(version) {
      const current = version || '—';
      const currentChannel = installedChannel(current);
      updateState({ currentVersion: current, installedChannel: currentChannel });
      if (!pendingUpdate && !operation) updateState({ updateChannel: currentChannel });
    },
    async setChannel(channel) {
      if (disposed || operation || state.value.phase === 'confirmingInstall' || state.value.phase === 'installing') return false;
      if (state.value.updateChannel === channel) return true;
      let cleanupError: unknown;
      try {
        await releasePendingUpdate();
      } catch (error) {
        cleanupError = error;
      }
      resetUpdateState(channel);
      if (cleanupError) setError(cleanupError, 'check');
      return true;
    },
    async checkForUpdates() {
      if (disposed || state.value.phase === 'confirmingInstall' || state.value.phase === 'installing' || pendingUpdate) return;
      await run(async () => {
        updateState({
          phase: 'checking',
          errorCode: undefined,
          latestVersion: undefined,
          releaseNotes: undefined,
          transition: undefined,
          blockedReason: undefined,
          downloadedBytes: 0,
          totalBytes: undefined,
          currentVersion: getCurrentVersion(),
          installedChannel: installedChannel(getCurrentVersion()),
        });
        try {
          const result = await adapter.check(state.value.updateChannel);
          if (disposed) {
            if (result.kind === 'available') await result.update.close();
            return;
          }
          if (result.kind === 'notConfigured') {
            updateState({ phase: 'unsupported', errorCode: 'unsupported' });
            return;
          }
          if (result.kind === 'channelEmpty') { updateState({ phase: 'channelEmpty' }); return; }
          if (result.kind === 'upToDate') {
            updateState({ phase: 'upToDate', currentVersion: result.installedVersion });
            return;
          }
          if (result.kind === 'blocked') {
            updateState({
              phase: 'blocked',
              currentVersion: result.installedVersion,
              latestVersion: result.targetVersion,
              blockedReason: result.reason,
            });
            return;
          }
          if (result.kind === 'failed') {
            setError(new UpdaterAdapterError(result.code), 'check');
            return;
          }

          const update = result.update;
          let transition: UpdateTransition;
          try { transition = classifyReleaseTransition(update.currentVersion || getCurrentVersion(), update.version); }
          catch {
            await update.close();
            setError(new UpdaterAdapterError('metadata'), 'check');
            return;
          }
          if (transition === 'sameVersion') {
            await update.close();
            updateState({ phase: 'upToDate', currentVersion: update.currentVersion || getCurrentVersion() });
            return;
          }
          if (transition === 'downgrade') {
            await update.close();
            updateState({
              phase: 'blocked',
              currentVersion: update.currentVersion || getCurrentVersion(),
              latestVersion: update.version,
              transition,
              blockedReason: 'compatibilityUnverified',
            });
            return;
          }
          pendingUpdate = update;
          updateState({
            phase: 'available',
            currentVersion: update.currentVersion || getCurrentVersion(),
            latestVersion: update.version,
            releaseNotes: update.body ?? undefined,
            transition,
          });
        } catch (error) { setError(error, 'check'); }
      });
    },
    async downloadUpdate() {
      const update = pendingUpdate;
      if (disposed || !update || state.value.phase !== 'available' || state.value.transition !== 'upgrade') return;
      await run(async () => {
        updateState({ phase: 'downloading', downloadedBytes: 0, totalBytes: undefined, errorCode: undefined });
        try {
          await update.download(event => {
            if (disposed) return;
            if (event.event === 'Started') {
              const total = event.data.contentLength;
              updateState({ downloadedBytes: 0, totalBytes: typeof total === 'number' && Number.isFinite(total) && total > 0 ? total : undefined });
            } else if (event.event === 'Progress') {
              const chunkLength = event.data.chunkLength;
              if (Number.isFinite(chunkLength) && chunkLength > 0) updateState({ downloadedBytes: state.value.downloadedBytes + chunkLength });
            }
          });
          if (disposed) { await releasePendingUpdate(); return; }
          updateState({ phase: 'downloaded' });
        } catch (error) {
          await releasePendingUpdate().catch(() => undefined);
          setError(error, 'download');
        }
      });
    },
    requestInstall() {
      if (!disposed && pendingUpdate && state.value.transition === 'upgrade' && state.value.phase === 'downloaded') updateState({ phase: 'confirmingInstall' });
    },
    cancelInstallRequest() {
      if (!disposed && pendingUpdate && state.value.phase === 'confirmingInstall') updateState({ phase: 'downloaded' });
    },
    async installConfirmed(platform) {
      const update = pendingUpdate;
      if (disposed || !update || state.value.phase !== 'confirmingInstall' || state.value.transition !== 'upgrade') return;
      await run(async () => {
        updateState({ phase: 'installing', errorCode: undefined });
        try {
          if (platform !== 'macos' && platform !== 'windows') throw new UpdaterAdapterError('unsupported');
          await update.install();
          if (platform === 'macos') await adapter.relaunch();
        } catch (error) {
          await releasePendingUpdate().catch(() => undefined);
          setError(error, 'install');
        }
      });
    },
    async dismiss() {
      if (disposed || operation || state.value.phase === 'installing' || state.value.phase === 'confirmingInstall') return;
      try {
        await releasePendingUpdate();
        resetUpdateState(state.value.updateChannel);
      } catch (error) { setError(error, 'check'); }
    },
    async dispose() {
      if (disposed) return;
      disposed = true;
      const closeAfterOperation = async () => { await releasePendingUpdate().catch(() => undefined); };
      if (operation) void operation.then(closeAfterOperation, closeAfterOperation);
      else await closeAfterOperation();
    },
  };
  return controller;
}
