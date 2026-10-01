import type { MonitorSnapshot } from "../../../contracts/v1/MonitorSnapshot";
import type { MonitorQualityStatus } from "../../../contracts/v1/MonitorQualityStatus";
import { createMockIpc } from "../ipc/mock";
import { fixtureError } from "./server-fixtures";
export function monitorFixture(connectionId: string, status: MonitorQualityStatus = 'ok', sampledAtMs = Date.now()): MonitorSnapshot {
  const known = status === 'ok' || status === 'stale';
  const quality = { status, sampledAtMs: status === 'warmingUp' ? null : sampledAtMs - (status === 'stale' ? 60000 : 0), collectionDurationMs: 24, errorCode: status === 'stale' || status === 'error' ? 'MONITOR_COLLECTION_FAILED' : null };
  return {
    connectionId,
    cpu: { usagePercent: known ? 24.5 : null, logicalCores: known ? 4 : null, quality: { ...quality } },
    memory: { usedBytes: known ? '4294967296' : null, totalBytes: known ? '8589934592' : null, availableBytes: known ? '4294967296' : null, usedPercent: known ? 50 : null, quality: { ...quality } },
    disk: { usedBytes: known ? '10737418240' : null, totalBytes: known ? '21474836480' : null, availableBytes: known ? '10737418240' : null, usedPercent: known ? 50 : null, source: known ? '/dev/vda1' : null, mount: known ? '/' : null, quality: { ...quality } },
    network: { receivedBytesPerSecond: known ? 1048576 : null, transmittedBytesPerSecond: known ? 262144 : null, interfaces: known ? [{ name: 'eth0', receivedBytes: '18446744073709551615', transmittedBytes: '9007199254740993', receivedBytesPerSecond: 1048576, transmittedBytesPerSecond: 262144 }] : [], quality: { ...quality } },
    load: { oneMinute: known ? .42 : null, fiveMinutes: known ? .35 : null, fifteenMinutes: known ? .29 : null, quality: { ...quality } },
    uptime: { seconds: known ? '90061' : null, quality: { ...quality } },
    system: { hostname: known ? 'fixture-linux' : null, os: known ? 'Debian GNU/Linux 12' : null, kernel: known ? '6.1-fixture' : null, architecture: known ? 'x86_64' : null, quality: { ...quality } },
  };
}
export function createMonitorMock(status: () => MonitorQualityStatus = () => 'unsupported', options: { failure?: () => boolean; historyFailure?: () => boolean; count?: (name: string) => void } = {}) {
  return createMockIpc({
    monitor_get_snapshot: ({ connectionId }) => { options.count?.('snapshot'); if (options.failure?.()) throw fixtureError('MONITOR_TIMEOUT', 'errors.monitorTimeout'); return monitorFixture(connectionId, status()); },
    monitor_refresh: ({ connectionId }) => { options.count?.('refresh'); if (options.failure?.()) throw fixtureError('MONITOR_TIMEOUT', 'errors.monitorTimeout'); return monitorFixture(connectionId, status()); },
    monitor_get_history: ({ connectionId, metric, limit, toMs }) => { options.count?.('history'); if (options.historyFailure?.()) throw fixtureError('MONITOR_COLLECTION_FAILED', 'errors.monitorCollectionFailed'); return { connectionId, metric, samples: Array.from({ length: Math.min(24, limit ?? 120) }, (_, index) => ({ sampledAtMs: (toMs ?? Date.now()) - (23 - index) * 5000, value: metric === 'cpuUsage' ? 15 + index % 7 * 2 : metric === 'networkReceiveRate' ? 100000 + index % 5 * 10000 : metric === 'networkTransmitRate' ? 30000 + index % 3 * 1000 : 25 + index % 4 })) }; },
    workspace_set_activity: ({ activeConnectionId, monitorVisible }) => { options.count?.(`activity:${activeConnectionId ?? 'none'}:${monitorVisible}`); },
  });
}
