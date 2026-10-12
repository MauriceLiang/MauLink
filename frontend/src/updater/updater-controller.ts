import { shallowRef } from 'vue';
import { UpdaterAdapterError } from './updater-adapter';
import type { PendingUpdate, UpdateDisplayState, UpdateErrorCode, UpdaterAdapter, UpdaterController } from './updater-types';

type Operation = 'check' | 'download' | 'install';

function classifyError(error: unknown, operation: Operation): UpdateErrorCode {
  if (error instanceof UpdaterAdapterError) return error.code;
  const message = error instanceof Error ? error.message.toLowerCase() : '';
  if (/signature|minisign|verify/.test(message)) return 'signature';
  if (/disk full|no space|storage|enospc/.test(message)) return 'storage';
  if (/permission|denied|401|403/.test(message)) return 'permission';
  if (/manifest|metadata|json|platform|version/.test(message)) return 'metadata';
  if (/endpoint|public key|pubkey|not configured/.test(message)) return 'unsupported';
  if (operation === 'install') return 'install';
  if (operation === 'check' || operation === 'download') return 'network';
  return 'unknown';
}

const initialState = (currentVersion: string, adapter: UpdaterAdapter): UpdateDisplayState => ({
  phase: adapter.kind === 'unsupported' ? 'unsupported' : 'idle',
  currentVersion,
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
    console.warn(`[updater:${stage}] operation failed (${errorCode})`, error);
    updateState({ phase: errorCode === 'unsupported' ? 'unsupported' : 'error', errorCode });
  }

  const controller: UpdaterController = {
    state,
    adapterKind: adapter.kind,
    setCurrentVersion(version) { updateState({ currentVersion: version || '—' }); },
    async checkForUpdates() {
      if (disposed || state.value.phase === 'confirmingInstall' || state.value.phase === 'installing' || pendingUpdate) return;
      await run(async () => {
        updateState({ phase: 'checking', errorCode: undefined, latestVersion: undefined, releaseNotes: undefined, downloadedBytes: 0, totalBytes: undefined, currentVersion: getCurrentVersion() });
        try {
          const update = await adapter.check();
          if (disposed) {
            if (update) await update.close();
            return;
          }
          if (!update) { updateState({ phase: 'upToDate' }); return; }
          pendingUpdate = update;
          updateState({ phase: 'available', currentVersion: update.currentVersion || getCurrentVersion(), latestVersion: update.version, releaseNotes: update.body ?? undefined });
        } catch (error) { setError(error, 'check'); }
      });
    },
    async downloadUpdate() {
      const update = pendingUpdate;
      if (disposed || !update || state.value.phase !== 'available') return;
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
      if (!disposed && pendingUpdate && state.value.phase === 'downloaded') updateState({ phase: 'confirmingInstall' });
    },
    cancelInstallRequest() {
      if (!disposed && pendingUpdate && state.value.phase === 'confirmingInstall') updateState({ phase: 'downloaded' });
    },
    async installConfirmed(platform) {
      const update = pendingUpdate;
      if (disposed || !update || state.value.phase !== 'confirmingInstall') return;
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
        updateState({ phase: adapter.kind === 'unsupported' ? 'unsupported' : 'idle', latestVersion: undefined, releaseNotes: undefined, downloadedBytes: 0, totalBytes: undefined, errorCode: adapter.kind === 'unsupported' ? 'unsupported' : undefined });
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
