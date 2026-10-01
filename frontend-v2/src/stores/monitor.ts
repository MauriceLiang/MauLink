import { ref, shallowRef } from "vue";
import type { MonitorSnapshot } from "../../../contracts/v1/MonitorSnapshot";
import type { MonitorHistoryMetric } from "../../../contracts/v1/MonitorHistoryMetric";
import type { MonitorHistorySample } from "../../../contracts/v1/MonitorHistorySample";
import type { AppError } from "../../../contracts/v1/AppError";
import type { createMonitorApi } from "../ipc/monitor";
import { mapError } from "../errors/mapper";
export const historyMetrics: Record<MonitorHistoryMetric, keyof Omit<MonitorSnapshot, 'connectionId'>> = { cpuUsage: 'cpu', memoryUsage: 'memory', diskUsage: 'disk', networkReceiveRate: 'network', networkTransmitRate: 'network', loadOneMinute: 'load' };
export function createMonitorStore(api: ReturnType<typeof createMonitorApi>) {
  const snapshot = shallowRef<MonitorSnapshot | null>(null);
  const histories = shallowRef<Partial<Record<MonitorHistoryMetric, MonitorHistorySample[]>>>({});
  const error = shallowRef<AppError | null>(null); const activityError = shallowRef<AppError | null>(null);
  const historyErrors = shallowRef<Partial<Record<MonitorHistoryMetric, AppError | null>>>({});
  const pending = ref(false); const refreshing = ref(false);
  let connectionId: string | null = null; let visible = false; let generation = 0; let disposed = false; let initialized = false;
  let timer: ReturnType<typeof setTimeout> | undefined; let lastHistory = -Infinity;
  let activityQueue = Promise.resolve();
  const current = (token: number, id: string) => !disposed && visible && token === generation && id === connectionId;
  async function history(token: number, id: string) {
    const now = Date.now();
    const results = await Promise.allSettled(Object.entries(historyMetrics).map(async ([metric, key]) => {
      const name = metric as MonitorHistoryMetric;
      if (snapshot.value?.[key].quality.status === 'unsupported') return { name, samples: [] };
      const page = await api.getHistory({ connectionId: id, metric: name, fromMs: now - 120000, toMs: now, limit: 120 });
      return { name, samples: page.samples.slice(-120) };
    }));
    if (!current(token, id)) return;
    const values = { ...histories.value }; const errors = { ...historyErrors.value };
    results.forEach((result, index) => {
      const metric = Object.keys(historyMetrics)[index] as MonitorHistoryMetric;
      if (result.status === 'fulfilled') { values[result.value.name] = result.value.samples; errors[metric] = null; }
      else errors[metric] = mapError(result.reason);
    });
    histories.value = values; historyErrors.value = errors; lastHistory = now;
  }
  async function read(token: number, id: string, manual = false) {
    if (!current(token, id) || pending.value) return false;
    clearTimeout(timer); pending.value = true; refreshing.value = manual;
    try {
      const value = await (manual ? api.refresh({ connectionId: id }) : api.getSnapshot({ connectionId: id }));
      if (!current(token, id)) return false;
      snapshot.value = value; error.value = null;
      if (manual || Date.now() - lastHistory >= 5000) await history(token, id);
      return true;
    } catch (reason) { if (current(token, id)) error.value = mapError(reason); return false; }
    finally {
      if (current(token, id)) { pending.value = false; refreshing.value = false; timer = setTimeout(() => { void read(token, id); }, 1000); }
    }
  }
  function activate(id: string | null, monitorVisible: boolean) {
    if (disposed || (initialized && connectionId === id && visible === monitorVisible)) return;
    initialized = true;
    const changed = connectionId !== id; connectionId = id; visible = monitorVisible && !!id;
    const token = ++generation; clearTimeout(timer); pending.value = false; refreshing.value = false; lastHistory = -Infinity;
    error.value = null; activityError.value = null;
    if (changed && id) { snapshot.value = null; histories.value = {}; historyErrors.value = {}; }
    // Serialize activity updates so a slow former workspace cannot become the final Core activity.
    const payload = { activeConnectionId: id, monitorVisible: visible };
    activityQueue = activityQueue.then(() => api.setActivity(payload)).catch(reason => { if (!disposed && token === generation) activityError.value = mapError(reason); });
    if (id && visible) void read(token, id);
  }
  const refresh = () => connectionId && visible ? read(generation, connectionId, true) : Promise.resolve(false);
  function dispose() { disposed = true; ++generation; clearTimeout(timer); }
  return { snapshot, histories, error, activityError, historyErrors, pending, refreshing, activate, refresh, dispose };
}
export type MonitorStore = ReturnType<typeof createMonitorStore>;
