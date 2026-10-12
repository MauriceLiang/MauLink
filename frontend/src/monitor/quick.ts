import type { MonitorQualityStatus } from "../../../contracts/v1/MonitorQualityStatus";

export type QuickCollectionState =
  | 'waiting'
  | 'ok'
  | 'partial'
  | 'stale'
  | 'unavailable'
  | 'disconnected';

export function quickCollectionState(
  ready: boolean,
  hasSnapshot: boolean,
  statuses: readonly MonitorQualityStatus[],
  fetchFailed: boolean,
): QuickCollectionState {
  if (!ready) return 'disconnected';
  if (!hasSnapshot) return fetchFailed ? 'unavailable' : 'waiting';
  if (fetchFailed || statuses.includes('stale')) return 'stale';
  if (statuses.length > 0 && statuses.every(status => status === 'error' || status === 'unsupported')) {
    return 'unavailable';
  }
  if (statuses.length > 0 && statuses.every(status => status === 'ok')) return 'ok';
  if (statuses.length > 0 && statuses.every(status => status === 'warmingUp')) return 'waiting';
  return 'partial';
}
