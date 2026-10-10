import { messages } from "../i18n/locale";
import { monitorMessages } from "../i18n/monitor";
import type { MonitorHistorySample } from "../../../contracts/v1/MonitorHistorySample";
import type { MonitorMetricQuality } from "../../../contracts/v1/MonitorMetricQuality";
import { formatSize as fileSize } from "../files/path";
const t = messages(monitorMessages);
export const formatSize = (value: string | null) => fileSize(value).replace(/\b([KMGTPE])B\b/g, "$1iB");
export const qualityLabel = (status: MonitorMetricQuality['status']) => t(status);
export const effectiveStatus = (quality?: MonitorMetricQuality, fetchFailed = false) => fetchFailed && quality?.status === 'ok' ? 'stale' : quality?.status ?? 'warmingUp';
export function metricValue(value: string, quality?: MonitorMetricQuality) {
  const state = quality?.status ?? 'warmingUp';
  return state === 'unsupported' || state === 'error' || state === 'warmingUp' ? qualityLabel(state) : value;
}
export const percent = (value?: number | null) => typeof value === 'number' && Number.isFinite(value) ? `${value.toFixed(1)}%` : '—';
export const loadValue = (value?: number | null) => typeof value === 'number' && Number.isFinite(value) ? value.toFixed(2) : '—';
export const rate = (value?: number | null) => typeof value === 'number' && Number.isFinite(value) && value >= 0 ? `${formatSize(String(Math.round(value)))}/s` : '—';
export function uptime(value?: string | null) {
  if (value == null) return '—';
  const seconds = Number(value); if (!Number.isFinite(seconds) || seconds < 0) return '—';
  const days = Math.floor(seconds / 86400); const hours = Math.floor(seconds % 86400 / 3600); const minutes = Math.floor(seconds % 3600 / 60);
  return days ? t('days', {days, hours}) : t('hours', {hours, minutes});
}
export function sparkline(samples: MonitorHistorySample[], width = 220, height = 42, domain?: readonly [number, number]) {
  const values = samples.slice(-120).filter(sample => Number.isFinite(sample.value)); if (!values.length) return '';
  const min = domain?.[0] ?? Math.min(...values.map(sample => sample.value)); const max = domain?.[1] ?? Math.max(...values.map(sample => sample.value));
  if (values.length === 1) return `M 0 ${height / 2} L ${width} ${height / 2}`;
  return values.map((sample, index) => {
    const ratio = max === min ? .5 : (Math.max(min, Math.min(max, sample.value)) - min) / (max - min);
    return `${index ? 'L' : 'M'} ${(index / (values.length - 1) * width).toFixed(1)} ${(height - 2 - ratio * (height - 4)).toFixed(1)}`;
  }).join(' ');
}
