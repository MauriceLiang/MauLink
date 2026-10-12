import type { Ref } from 'vue';

export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
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

export interface UpdateDisplayState {
  phase: UpdatePhase;
  currentVersion: string;
  latestVersion?: string;
  releaseNotes?: string;
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

export interface UpdaterAdapter {
  readonly kind: 'native' | 'mock' | 'unsupported';
  check(): Promise<PendingUpdate | null>;
  relaunch(): Promise<void>;
}

export interface UpdaterController {
  readonly state: Readonly<Ref<UpdateDisplayState>>;
  readonly adapterKind: UpdaterAdapter['kind'];
  setCurrentVersion(version: string): void;
  checkForUpdates(): Promise<void>;
  downloadUpdate(): Promise<void>;
  requestInstall(): void;
  cancelInstallRequest(): void;
  installConfirmed(platform: 'macos' | 'windows' | string): Promise<void>;
  dismiss(): Promise<void>;
  dispose(): Promise<void>;
}
