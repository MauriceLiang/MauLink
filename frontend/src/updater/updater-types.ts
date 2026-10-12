import type { Ref } from 'vue';
import type { UpdateChannel } from '../../../contracts/v1/UpdateChannel';

export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'channelEmpty'
  | 'upToDate'
  | 'available'
  | 'blocked'
  | 'downloading'
  | 'downloaded'
  | 'confirmingInstall'
  | 'installing'
  | 'error'
  | 'unsupported';

export type UpdateErrorCode =
  | 'network'
  | 'metadata'
  | 'signature'
  | 'storage'
  | 'permission'
  | 'install'
  | 'unsupported'
  | 'unknown';

export type UpdateTransition = 'upgrade' | 'downgrade' | 'sameVersion';
export type UpdateBlockedReason = 'compatibilityUnverified' | 'schemaIncompatible' | 'backupFailed' | 'installerNotValidated';
export type { UpdateChannel };

export interface UpdateDisplayState {
  phase: UpdatePhase;
  currentVersion: string;
  installedChannel: UpdateChannel;
  updateChannel: UpdateChannel;
  latestVersion?: string;
  releaseNotes?: string;
  transition?: UpdateTransition;
  blockedReason?: UpdateBlockedReason;
  downloadedBytes: number;
  totalBytes?: number;
  errorCode?: UpdateErrorCode;
}

export type UpdateDownloadEvent =
  | { event: 'Started'; data: { contentLength?: number } }
  | { event: 'Progress'; data: { chunkLength: number } }
  | { event: 'Finished' };

export interface PendingUpdate {
  currentVersion: string;
  version: string;
  body?: string | null;
  date?: string | null;
  download(onEvent: (event: UpdateDownloadEvent) => void): Promise<void>;
  install(): Promise<void>;
  close(): Promise<void>;
}

export type UpdateCheckResponse =
  | { kind: 'notConfigured' }
  | { kind: 'channelEmpty'; channel: UpdateChannel }
  | { kind: 'upToDate'; channel: UpdateChannel; installedVersion: string }
  | {
      kind: 'available';
      channel: UpdateChannel;
      installedVersion: string;
      targetVersion: string;
      transition: 'upgrade';
      releaseNotes?: string | null;
      publishedAt?: string | null;
    }
  | {
      kind: 'blocked';
      channel: UpdateChannel;
      installedVersion: string;
      targetVersion: string;
      reason: UpdateBlockedReason;
    }
  | { kind: 'failed'; code: UpdateErrorCode };

export type UpdateOperationResult =
  | { kind: 'completed' }
  | { kind: 'failed'; code: UpdateErrorCode };

export type UpdaterCheckResult =
  | { kind: 'notConfigured' }
  | { kind: 'channelEmpty' }
  | { kind: 'upToDate'; installedVersion: string }
  | { kind: 'available'; update: PendingUpdate }
  | { kind: 'blocked'; installedVersion: string; targetVersion: string; reason: UpdateBlockedReason }
  | { kind: 'failed'; code: UpdateErrorCode };

export interface UpdaterAdapter {
  readonly kind: 'native' | 'mock' | 'unsupported';
  check(channel: UpdateChannel): Promise<UpdaterCheckResult>;
  relaunch(): Promise<void>;
}

export interface UpdaterController {
  readonly state: Readonly<Ref<UpdateDisplayState>>;
  readonly adapterKind: UpdaterAdapter['kind'];
  setCurrentVersion(version: string): void;
  setChannel(channel: UpdateChannel): Promise<boolean>;
  checkForUpdates(): Promise<void>;
  downloadUpdate(): Promise<void>;
  requestInstall(): void;
  cancelInstallRequest(): void;
  installConfirmed(platform: 'macos' | 'windows' | string): Promise<void>;
  dismiss(): Promise<void>;
  dispose(): Promise<void>;
}
