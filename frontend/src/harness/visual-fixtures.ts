import type { GeoIpDatabaseStatus } from '../../../contracts/v1/GeoIpDatabaseStatus';
import type { Channel } from '@tauri-apps/api/core';
import type { ConnectionSnapshot } from '../../../contracts/v1/ConnectionSnapshot';
import type { BackgroundImageAsset } from '../../../contracts/v1/BackgroundImageAsset';
import type { RemoteFileEntry } from '../../../contracts/v1/RemoteFileEntry';
import type { SettingsRecord } from '../../../contracts/v1/SettingsRecord';
import type { SftpTransferSnapshot } from '../../../contracts/v1/SftpTransferSnapshot';
import type { TerminalChunk } from '../../../contracts/v1/TerminalChunk';
import type { TerminalSnapshot } from '../../../contracts/v1/TerminalSnapshot';
import type { TerminalThemeMode } from '../../../contracts/v1/TerminalThemeMode';
import type { AccentColor } from '../../../contracts/v1/AccentColor';
import type { UiDensity } from '../../../contracts/v1/UiDensity';
import type { IpcTransport } from '../ipc/client';
import { createMockIpc } from '../ipc/mock';
import { defaultSettings } from '../terminal/preferences';
import { encodeBytesBase64 } from '../terminal/codec';
import { shellAppInfo, shellGroups, shellServers } from './shell-fixtures';
import { fixtureError } from './server-fixtures';
import { monitorFixture } from './monitor-fixtures';
import { networkFixture, preflightFixture } from './network-fixtures';
import { createCredentialRevealMock } from './credential-reveal-fixtures';
import cases from '../../visual/cases.json';
import fileData from '../../visual/files.json';

export const visualEpoch = 1790812800000;
export const visualBackgroundImageId = '00000000-0000-4000-8000-000000000003';
const visualBackgroundImageUrl = new URL('../assets/app-icon-dark.png', import.meta.url).href;
const visualBackgroundImage: BackgroundImageAsset = { id: visualBackgroundImageId, fileName: 'visual-terminal-background.png', mediaType: 'image/png', width: 256, height: 256, byteLength: 51669, createdAtMs: visualEpoch };
const terminalThemeModes: Record<string, TerminalThemeMode> = {
  'terminal-light': 'light', 'terminal-dark': 'dark', 'terminal-custom': 'customColor', 'terminal-image': 'image', 'settings-terminal-image': 'image',
};
export interface VisualConfig { page: string; theme: 'light' | 'dark'; locale: 'zh-CN' | 'en'; accentColor: AccentColor; customAccentColor: string | null; uiDensity: UiDensity; }
export function visualConfig(params: URLSearchParams): VisualConfig {
  const page = params.get('page') ?? 'servers';
  const accent = params.get('accentColor');
  const accentColor: AccentColor = ['blue', 'indigo', 'purple', 'green', 'orange', 'red', 'custom'].includes(accent ?? '') ? accent as AccentColor : 'blue';
  const customAccentColor = params.get('customAccentColor');
  return {
    page: cases.some(value => value.page === page) ? page : 'servers',
    theme: params.get('theme') === 'dark' ? 'dark' : 'light',
    locale: params.get('locale') === 'en' ? 'en' : 'zh-CN',
    accentColor,
    customAccentColor: accentColor === 'custom' && /^#[\da-f]{6}$/i.test(customAccentColor ?? '') ? customAccentColor : null,
    uiDensity: params.get('uiDensity') === 'compact' ? 'compact' : 'standard',
  };
}
// Each visual document gets isolated front-end preferences; persistent browser data is untouched.
export function createVisualStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; }, key: index => [...values.keys()][index] ?? null,
    getItem: key => values.get(key) ?? null, setItem: (key, value) => { values.set(key, value); },
    removeItem: key => { values.delete(key); }, clear: () => values.clear(),
  };
}
export const visualFiles: RemoteFileEntry[] = fileData.map(({name, fileType, sizeBytes}) => ({ name, path: `/opt/app/${name}`, fileType: fileType === 'directory' ? 'directory' : 'file', sizeBytes, modifiedAtMs: visualEpoch, isSymlink: false, permissions: 420 }));
export function visualTransfers(connectionId: string): SftpTransferSnapshot[] {
  const base = { connectionId, direction: 'upload' as const, totalBytes: '104857600', bytesPerSecond: null, remainingSeconds: null, error: null, cleanupRequired: false, temporaryPath: null };
  return [
    { ...base, transferId: 'visual-transfer-1', fileName: 'config.yml', finalPath: '/opt/app/config.yml', transferredBytes: '75497472', bytesPerSecond: 14680064, remainingSeconds: 2n, state: 'transferring' },
    { ...base, transferId: 'visual-transfer-2', fileName: 'backup.sql', finalPath: '/opt/app/backup.sql', transferredBytes: '39845888', state: 'failed', error: fixtureError('LOCAL_DISK_FULL', 'errors.localDiskFull') },
    { ...base, transferId: 'visual-transfer-3', fileName: 'landing-page.zip', finalPath: '/opt/app/landing-page.zip', transferredBytes: '104857600', state: 'completed' },
  ];
}
export const visualTranscript = 'Last login: Sun Sep 28 09:12:04 on ttys001\r\n$ ls -la\r\ntotal 48\r\n-rw-r--r-- 1 root root 2154 Sep 28 08:59 package.json\r\n-rwxr-xr-x 1 root root 4830 Sep 27 18:20 deploy.sh\r\n$ ./deploy.sh --env production\r\n\x1b[32m[ ok ] Dependencies installed\r\n[ ok ] Release 2.14.0 activated\x1b[0m\r\n$ ';

// Every visible value is fixed. Ordinary production stores/pollers still run against this isolated transport.
export function createVisualMock(config: VisualConfig) {
  const servers = config.page === 'empty' ? [] : structuredClone(shellServers);
  if (config.page === 'credential-reveal' && servers[0]) servers[0].hasSavedCredential = true;
  if (config.page === 'server-overview' && servers[0]) {
    servers[0].hasSavedCredential = true;
    servers[0].host = '8.8.8.8';
    servers[0].createdAtMs = visualEpoch - 30 * 24 * 60 * 60 * 1000;
    servers[0].updatedAtMs = visualEpoch - 24 * 60 * 60 * 1000;
  }
  const terminalThemeMode = terminalThemeModes[config.page] ?? defaultSettings.terminalThemeMode;
  const terminalCustomColors = config.page === 'terminal-custom' ? { background: '#241A36', foreground: '#F4ECFF', cursor: '#C084FC', selection: '#8B5CF6' } : defaultSettings.terminalCustomColors;
  const terminalBackgroundImage = { ...defaultSettings.terminalBackgroundImage, imageId: terminalThemeMode === 'image' ? visualBackgroundImageId : null };
  let settings: SettingsRecord = { value: { ...defaultSettings, theme: config.theme, language: config.locale, accentColor: config.accentColor, customAccentColor: config.customAccentColor, uiDensity: config.uiDensity, terminalThemeMode, terminalCustomColors, terminalBackgroundImage }, revision: 1, updatedAtMs: visualEpoch };
  let connection: ConnectionSnapshot = { connectionId: 'visual-connection', serverId: 'web-01', mode: 'workspace', state: config.page === 'server-overview' ? 'closed' : 'ready', hostKeyChallenge: null, authenticationChallenge: null, negotiatedAlgorithms: null, error: null, createdAtMs: visualEpoch, updatedAtMs: visualEpoch };
  const terminals = new Map<string, TerminalSnapshot>(); let terminalNumber = 0; let output: Channel<TerminalChunk> | undefined;
  const files = structuredClone(visualFiles); const tasks = ['transfer', 'disconnect-transfer'].includes(config.page) ? visualTransfers(connection.connectionId) : [];
  let geoip: GeoIpDatabaseStatus = { location: null, asn: null, automaticUpdates: false, updateIntervalDays: 30, lastCheckedAtMs: null, lastError: null };
  const databaseInfo = (kind: string, source: 'dbIp' | 'local') => ({ fileName: `${source}-${kind}.mmdb`, databaseType: `${source}-${kind}`, buildAtMs: visualEpoch, source, available: true });
  const snapshot = () => structuredClone(connection);
  const transport = createMockIpc({
    app_get_info: () => shellAppInfo,
    group_list: () => config.page === 'empty' ? [] : structuredClone(shellGroups),
    server_list: () => ({ items: structuredClone(servers), nextCursor: null }),
    server_appearance_list: () => [],
    server_appearance_get: ({ serverId }) => ({
      serverId,
      labelColor: null,
      environment: null,
      terminalOverrideEnabled: false,
      terminalAppearance: {
        themeMode: 'followApp',
        customColors: { background: '#111318', foreground: '#EAECF0', cursor: '#3B82F6', selection: '#3B82F6' },
        backgroundImage: { imageId: null, fit: 'cover', position: 'center', imageOpacity: 100, overlayKind: 'dark', overlayOpacity: 45, blurPx: 0 },
      },
      revision: 0,
      updatedAtMs: visualEpoch,
    }),
    server_get: ({ id }) => { const server = servers.find(value => value.id === id); if (!server) throw fixtureError('RESOURCE_NOT_FOUND', 'errors.serverNotFound'); return structuredClone(server); },
    server_runtime_stats_get: ({ serverId }) => ({ serverId, lastSuccessAtMs: config.page === 'server-overview' ? visualEpoch - 60 * 60 * 1000 : null, lastFailureAtMs: null, lastPreflightAtMs: config.page === 'server-overview' ? visualEpoch - 2 * 60 * 60 * 1000 : null, lastPreflightLatencyMs: config.page === 'server-overview' ? 47 : null, lastFailureCode: null, updatedAtMs: visualEpoch }),
    background_image_get: ({ imageId }) => {
      if (imageId !== visualBackgroundImageId) throw fixtureError('RESOURCE_NOT_FOUND', 'errors.terminalBackgroundImageNotFound');
      return { asset: structuredClone(visualBackgroundImage), localPath: visualBackgroundImageUrl };
    },
    host_key_get: ({ host, port }) => config.page === 'server-overview' ? ({ normalizedHost: host.toLowerCase(), port, algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:visual-saved-host-fingerprint', revision: 1, trustedAtMs: visualEpoch - 7 * 24 * 60 * 60 * 1000 }) : null,
    geoip_database_get: () => structuredClone(geoip),
    geoip_database_configure: ({ updateIntervalDays }) => { geoip.updateIntervalDays = updateIntervalDays; return structuredClone(geoip); },
    geoip_database_update: () => { geoip = { ...geoip, location: databaseInfo('City', 'dbIp'), asn: databaseInfo('ASN', 'dbIp'), automaticUpdates: true, lastCheckedAtMs: visualEpoch }; return structuredClone(geoip); },
    geoip_database_import: () => { geoip = { ...geoip, location: databaseInfo('City', 'local'), automaticUpdates: false }; return structuredClone(geoip); },
    geoip_database_delete: () => { geoip = { ...geoip, location: null, asn: null, automaticUpdates: false, lastCheckedAtMs: null, lastError: null }; return structuredClone(geoip); },
    local_file_select: ({ purpose }) => purpose === 'geoIpDatabase' ? { token: 'visual-mmdb', displayName: 'local-City.mmdb', purpose, expiresAtMs: 4102444800000 } : null,
    network_inspect: ({ host, detailed, language }) => {
      const result = networkFixture(host, detailed);
      if (detailed && result.scope === 'public' && geoip.location) {
        result.geo = language === 'en'
          ? { countryCode: 'US', countryName: 'United States', region: 'California', city: 'Mountain View' }
          : { countryCode: 'US', countryName: '美国', region: '加利福尼亚州', city: '山景城' };
        result.asn = geoip.asn ? 'AS15169' : null; result.organization = geoip.asn ? 'Google LLC' : null;
        result.databaseUpdatedAtMs = visualEpoch; result.databaseSource = geoip.asn?.source === 'dbIp' ? 'dbIp' : geoip.location.source;
      }
      return result;
    },
    connection_preflight: ({ host }) => preflightFixture(host),
    settings_get: () => structuredClone(settings),
    settings_update: ({ expectedRevision, value }) => { if (expectedRevision !== settings.revision) throw fixtureError('REVISION_CONFLICT', 'errors.revisionConflict'); settings = { value: structuredClone(value), revision: settings.revision + 1, updatedAtMs: visualEpoch }; return structuredClone(settings); },
    connection_start: () => {
      if (config.page === 'host-key' || config.page === 'host-key-changed') connection = { ...connection, state: 'awaitingHostTrust', hostKeyChallenge: { challengeId: 'visual-host-key', connectionId: connection.connectionId, host: '192.168.1.20', port: 22, algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:visual-current-fingerprint', previousFingerprintSha256: config.page === 'host-key-changed' ? 'SHA256:visual-previous-fingerprint' : null, previousRevision: config.page === 'host-key-changed' ? 1 : null, expiresAtMs: 4102444800000 } };
      else if (config.page === 'authentication') connection = { ...connection, state: 'awaitingCredentials', authenticationChallenge: { challengeId: 'visual-auth', connectionId: connection.connectionId, credentialKind: 'password', expiresAtMs: 4102444800000 } };
      else if (config.page === 'connection-error') connection = { ...connection, state: 'failed', error: { ...fixtureError('CONNECTION_REFUSED', 'errors.connectionRefused'), retryable: true, stage: 'connecting' } };
      return snapshot();
    },
    connection_get: snapshot,
    connection_cancel: () => { connection = { ...connection, state: 'cancelled', hostKeyChallenge: null, authenticationChallenge: null }; return snapshot(); },
    connection_disconnect: () => { connection = { ...connection, state: 'closed' }; return snapshot(); },
    terminal_open: ({ connectionId, columns, rows }) => {
      const terminalId = `visual-terminal-${++terminalNumber}`; const streamId = `visual-stream-${terminalNumber}`;
      terminals.set(terminalId, { terminalId, streamId, connectionId, size: { columns, rows, pixelWidth: null, pixelHeight: null }, state: 'running', error: null, exitStatus: null, exitSignal: null, createdAtMs: visualEpoch, updatedAtMs: visualEpoch });
      const bytes = new TextEncoder().encode(visualTranscript);
      output?.onmessage({ terminalId, streamId, seq: '1', byteLength: bytes.length, dataBase64: encodeBytesBase64(bytes) });
      return { terminalId, streamId };
    },
    terminal_get: ({ terminalId }) => structuredClone(terminals.get(terminalId)!),
    terminal_resize: ({ terminalId, ...size }) => { terminals.get(terminalId)!.size = size; return size; },
    terminal_write: ({ inputSeq }) => ({ inputSeq, duplicate: false }), terminal_ack: () => undefined,
    terminal_close: ({ terminalId }) => { const value = terminals.get(terminalId)!; value.state = 'closed'; return structuredClone(value); },
    sftp_list_start: ({ path }) => ({ path: path === '.' || path === '' ? '/opt/app' : path, entries: structuredClone(files), cursorId: null }),
    sftp_list_close: () => undefined,
    sftp_stat: ({ path }) => structuredClone(files.find(value => value.path === path)!),
    sftp_transfer_list: () => structuredClone(tasks),
    sftp_transfer_get: ({ transferId }) => structuredClone(tasks.find(value => value.transferId === transferId)!),
    monitor_get_snapshot: ({ connectionId }) => monitorFixture(connectionId, config.page === 'monitor-unavailable' ? 'unsupported' : 'ok', visualEpoch),
    monitor_refresh: ({ connectionId }) => monitorFixture(connectionId, config.page === 'monitor-unavailable' ? 'unsupported' : 'ok', visualEpoch),
    monitor_get_history: ({ connectionId, metric }) => ({ connectionId, metric, samples: Array.from({ length: 24 }, (_, index) => ({ sampledAtMs: visualEpoch - (23 - index) * 5000, value: metric === 'cpuUsage' ? 15 + index % 7 * 2 : metric === 'networkReceiveRate' ? 100000 + index % 5 * 10000 : metric === 'networkTransmitRate' ? 30000 + index % 3 * 1000 : 25 + index % 4 })) }),
    workspace_set_activity: () => undefined,
  });
  const credentialReveal = createCredentialRevealMock();
  const credentialRevealCommands = new Set(['reveal_policy_get', 'reveal_policy_enable_protected', 'reveal_policy_enable_direct', 'reveal_policy_set_deny', 'reveal_policy_change_password', 'reveal_policy_recover', 'credential_reveal']);
  const clientTransport: IpcTransport = { invoke<T>(command: string, args?: Record<string, unknown>) {
    if (command === 'terminal_open') output = args?.outputChannel as Channel<TerminalChunk>;
    if (credentialRevealCommands.has(command)) return credentialReveal.transport.invoke<T>(command, args);
    return transport.invoke<T>(command, args);
  } };
  return clientTransport;
}
