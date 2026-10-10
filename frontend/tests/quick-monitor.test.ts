import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import type { MonitorHistoryMetric } from '../../contracts/v1/MonitorHistoryMetric';
import type { MonitorHistorySample } from '../../contracts/v1/MonitorHistorySample';
import type { MonitorQualityStatus } from '../../contracts/v1/MonitorQualityStatus';
import type { MonitorSnapshot } from '../../contracts/v1/MonitorSnapshot';
import { locale } from '../src/i18n/locale';
import { createIpcClient } from '../src/ipc/client';
import { createMockIpc } from '../src/ipc/mock';
import { createMonitorApi } from '../src/ipc/monitor';
import { createMonitorStore } from '../src/stores/monitor';
import { quickCollectionState } from '../src/monitor/quick';
import { fixtureError } from '../src/harness/server-fixtures';
import { monitorFixture } from '../src/harness/monitor-fixtures';
import QuickMonitorPanel from '../src/components/monitor/QuickMonitorPanel.vue';

const stores: ReturnType<typeof createMonitorStore>[] = [];
const wrappers: VueWrapper[] = [];
const allOk = ['ok', 'ok', 'ok', 'ok', 'ok'] as const;
const fixtureHistory = (metric: MonitorHistoryMetric): MonitorHistorySample[] => [
  { sampledAtMs: 1, value: metric === 'cpuUsage' ? 20 : 1 },
  { sampledAtMs: 2, value: metric === 'cpuUsage' ? 24 : 2 },
];

afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  stores.splice(0).forEach(store => store.dispose());
  locale.value = 'zh-CN';
  vi.useRealTimers();
  document.body.innerHTML = '';
});

function controller(options: {
  status?: MonitorQualityStatus;
  snapshot?: (connectionId: string) => MonitorSnapshot;
  history?: (metric: MonitorHistoryMetric) => MonitorHistorySample[];
  historyFailures?: MonitorHistoryMetric[];
  failInitially?: boolean;
} = {}) {
  let fail = options.failInitially ?? false;
  const getSnapshot = vi.fn(({ connectionId }: { connectionId: string }) => {
    if (fail) throw fixtureError('MONITOR_TIMEOUT', 'errors.monitorTimeout');
    return options.snapshot?.(connectionId) ?? monitorFixture(connectionId, options.status ?? 'ok', 1_700_000_000_000);
  });
  const history = vi.fn(({ connectionId, metric, limit }: { connectionId: string; metric: MonitorHistoryMetric; limit?: number | null }) => {
    if (options.historyFailures?.includes(metric)) throw fixtureError('MONITOR_COLLECTION_FAILED', 'errors.monitorCollectionFailed');
    const samples = options.history?.(metric) ?? fixtureHistory(metric);
    return { connectionId, metric, samples: samples.slice(-(limit ?? 120)) };
  });
  const store = createMonitorStore(createMonitorApi(createIpcClient(createMockIpc({
    monitor_get_snapshot: getSnapshot,
    monitor_refresh: getSnapshot,
    monitor_get_history: history,
    workspace_set_activity: () => undefined,
  }))));
  stores.push(store);
  store.activate('conn-a', true);
  return { store, getSnapshot, history, setFailure: (value: boolean) => { fail = value; } };
}

async function panel(
  store: ReturnType<typeof createMonitorStore>,
  props: { connectionId?: string; ready?: boolean } = {},
) {
  const wrapper = mount(QuickMonitorPanel, {
    props: { store, connectionId: props.connectionId ?? 'conn-a', ready: props.ready ?? true },
  });
  wrappers.push(wrapper);
  await flushPromises();
  return wrapper;
}

describe('quickCollectionState', () => {
  it('prioritizes disconnect even with a valid snapshot or failed fetch', () => {
    expect(quickCollectionState(false, true, allOk, false)).toBe('disconnected');
    expect(quickCollectionState(false, true, allOk, true)).toBe('disconnected');
  });
  it('distinguishes waiting from unavailable when no snapshot exists', () => {
    expect(quickCollectionState(true, false, [], false)).toBe('waiting');
    expect(quickCollectionState(true, false, [], true)).toBe('unavailable');
  });
  it('reports all-ok and all-warming-up snapshots accurately', () => {
    expect(quickCollectionState(true, true, allOk, false)).toBe('ok');
    expect(quickCollectionState(true, true, ['warmingUp', 'warmingUp', 'warmingUp', 'warmingUp', 'warmingUp'], false)).toBe('waiting');
  });
  it('marks all unsupported/error combinations unavailable', () => {
    expect(quickCollectionState(true, true, ['unsupported', 'unsupported', 'unsupported', 'unsupported', 'unsupported'], false)).toBe('unavailable');
    expect(quickCollectionState(true, true, ['error', 'error', 'error', 'error', 'error'], false)).toBe('unavailable');
    expect(quickCollectionState(true, true, ['unsupported', 'error', 'unsupported', 'error', 'error'], false)).toBe('unavailable');
  });
  it('prioritizes stale data and reports mixed availability as partial', () => {
    expect(quickCollectionState(true, true, ['stale', 'ok', 'ok', 'ok', 'ok'], false)).toBe('stale');
    expect(quickCollectionState(true, true, allOk, true)).toBe('stale');
    expect(quickCollectionState(true, true, ['ok', 'unsupported', 'ok', 'ok', 'ok'], false)).toBe('partial');
    expect(quickCollectionState(true, true, ['warmingUp', 'ok', 'ok', 'ok', 'ok'], false)).toBe('partial');
  });
});

describe('Quick Monitor presentation', () => {
  it('renders a CPU hero and compact semantic rows for each secondary metric', async () => {
    const { store } = controller();
    const wrapper = await panel(store);
    expect(wrapper.attributes('aria-label')).toBe('监控概览');
    expect(wrapper.get('.quick-monitor-state').attributes('data-state')).toBe('ok');
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('24.5%');
    expect(wrapper.get('[data-metric="memory"] dd').text()).toBe('50.0%');
    expect(wrapper.get('[data-metric="disk"] dd').text()).toBe('50.0%');
    expect(wrapper.get('[data-metric="network"] [data-direction="receive"]').text()).toBe('1.0 MiB/s');
    expect(wrapper.get('[data-metric="network"] [data-direction="transmit"]').text()).toBe('256.0 KiB/s');
    expect(wrapper.get('[data-metric="load"] dd').text()).toBe('0.42');
    expect(wrapper.findAll('.quick-monitor-secondary .quick-monitor-metric')).toHaveLength(4);
  });

  it('does not display a snapshot from a different connection', async () => {
    const { store } = controller();
    const wrapper = await panel(store, { connectionId: 'conn-b' });
    expect(wrapper.attributes('data-collection-state')).toBe('waiting');
    expect(wrapper.text()).not.toContain('24.5%');
    expect(wrapper.text()).not.toContain('1.0 MiB/s');
    expect(wrapper.text()).not.toContain('Updated');
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(false);
  });

  it('preserves real zero values and renders missing values as dashes', async () => {
    const { store } = controller({ snapshot: connectionId => {
      const value = monitorFixture(connectionId, 'ok', 1_700_000_000_000);
      return {
        ...value,
        cpu: { ...value.cpu, usagePercent: 0 },
        memory: { ...value.memory, usedPercent: 0 },
        disk: { ...value.disk, usedPercent: 0 },
        network: { ...value.network, receivedBytesPerSecond: 0, transmittedBytesPerSecond: 0 },
        load: { ...value.load, oneMinute: 0 },
      };
    } });
    const wrapper = await panel(store);
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('0.0%');
    expect(wrapper.get('[data-metric="memory"] dd').text()).toBe('0.0%');
    expect(wrapper.get('[data-direction="receive"]').text()).toBe('0 B/s');
    expect(wrapper.get('[data-direction="transmit"]').text()).toBe('0 B/s');
    expect(wrapper.get('[data-metric="load"] dd').text()).toBe('0.00');
  });

  it('does not replace null metric values with zero', async () => {
    const { store } = controller({ snapshot: connectionId => {
      const value = monitorFixture(connectionId, 'ok', 1_700_000_000_000);
      return {
        ...value,
        cpu: { ...value.cpu, usagePercent: null },
        memory: { ...value.memory, usedPercent: null },
        disk: { ...value.disk, usedPercent: null },
        network: { ...value.network, receivedBytesPerSecond: null, transmittedBytesPerSecond: null },
        load: { ...value.load, oneMinute: null },
      };
    } });
    const wrapper = await panel(store);
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('—');
    expect(wrapper.get('[data-metric="memory"] dd').text()).toBe('—');
    expect(wrapper.get('[data-metric="disk"] dd').text()).toBe('—');
    expect(wrapper.get('[data-direction="receive"]').text()).toBe('—');
    expect(wrapper.get('[data-metric="load"] dd').text()).toBe('—');
  });

  it.each(['warmingUp', 'unsupported'] as const)('renders %s without fake zero metrics or trends', async status => {
    const { store } = controller({ status });
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe(status === 'warmingUp' ? 'waiting' : 'unavailable');
    expect(wrapper.get('.quick-monitor-hero-value').text()).not.toContain('0%');
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('正在采样');
    expect(wrapper.text()).toContain(status === 'warmingUp' ? '暂无数据' : '不支持');
  });

  it('keeps the last valid metrics as stale while the next snapshot warms up', async () => {
    let status: MonitorQualityStatus = 'ok';
    const { store } = controller({ snapshot: connectionId => monitorFixture(connectionId, status, 1_700_000_000_000) });
    const wrapper = await panel(store);
    status = 'warmingUp';
    await store.refresh();

    expect(wrapper.attributes('data-collection-state')).toBe('stale');
    expect(wrapper.get('.quick-monitor-hero').attributes('data-quality')).toBe('stale');
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('24.5%');
    expect(wrapper.text()).not.toContain('正在采样');
  });

  it('keeps other valid values visible when one metric fails', async () => {
    const { store } = controller({ snapshot: connectionId => {
      const value = monitorFixture(connectionId, 'ok', 1_700_000_000_000);
      return { ...value, cpu: { ...value.cpu, usagePercent: null, quality: { ...value.cpu.quality, status: 'error' } } };
    } });
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe('partial');
    expect(wrapper.get('.quick-monitor-hero').attributes('data-quality')).toBe('error');
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('暂不可用');
    expect(wrapper.get('[data-metric="memory"] dd').text()).toBe('50.0%');
  });

  it('marks old snapshot data stale when the snapshot fetch fails', async () => {
    const { store, setFailure } = controller();
    await flushPromises();
    setFailure(true);
    await store.refresh();
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe('stale');
    expect(wrapper.get('.quick-monitor-state').text()).toContain('数据已过期');
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('24.5%');
    expect(wrapper.get('.quick-monitor-hero').attributes('data-quality')).toBe('stale');
  });

  it('shows disconnected and last-sample text after disconnect, and disables the Full Monitor CTA', async () => {
    const { store } = controller();
    await flushPromises();
    const wrapper = await panel(store, { ready: false });
    expect(wrapper.attributes('data-collection-state')).toBe('disconnected');
    expect(wrapper.get('.quick-monitor-time').text()).toContain('最后采样');
    expect(wrapper.get('.monitor-open-full').element).toHaveProperty('disabled', true);
    await wrapper.get('.monitor-open-full').trigger('click');
    expect(wrapper.emitted('full')).toBeUndefined();
  });

  it('shows unavailable without data when the initial snapshot fetch fails', async () => {
    const { store } = controller({ failInitially: true });
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe('unavailable');
    expect(wrapper.text()).not.toContain('24.5%');
    expect(wrapper.get('.monitor-open-full').element).toHaveProperty('disabled', false);
  });

  it('shows disconnected without old metrics when no snapshot exists after disconnect', async () => {
    const { store } = controller({ failInitially: true });
    const wrapper = await panel(store, { ready: false });
    expect(wrapper.attributes('data-collection-state')).toBe('disconnected');
    expect(wrapper.text()).not.toContain('24.5%');
    expect(wrapper.get('.quick-monitor-time').text()).toBe('暂无数据');
    expect(wrapper.get('.monitor-open-full').element).toHaveProperty('disabled', true);
  });

  it('shows a real CPU trend only when at least two valid history samples exist', async () => {
    const { store } = controller();
    const wrapper = await panel(store);
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(true);
    expect(wrapper.get('svg.monitor-chart').attributes('aria-label')).toBe('CPU 最近 2 分钟使用率');
  });

  it('shows a trend placeholder for missing or invalid history samples', async () => {
    const { store } = controller({ history: () => [{ sampledAtMs: 1, value: Number.NaN }] });
    const wrapper = await panel(store);
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(false);
    expect(wrapper.get('.quick-monitor-trend-placeholder').text()).toBe('暂无趋势数据');
  });

  it('keeps the current CPU value when its history request fails', async () => {
    const { store } = controller({ historyFailures: ['cpuUsage'] });
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe('ok');
    expect(wrapper.get('.quick-monitor-hero-value').text()).toBe('24.5%');
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(false);
    expect(wrapper.get('.quick-monitor-trend-placeholder').text()).toBe('趋势暂不可用');
  });

  it('keeps the CPU trend when only another metric history request fails', async () => {
    const { store } = controller({ historyFailures: ['memoryUsage'] });
    const wrapper = await panel(store);
    expect(wrapper.attributes('data-collection-state')).toBe('ok');
    expect(wrapper.find('svg.monitor-chart').exists()).toBe(true);
  });

  it('labels server receive and transmit directions accurately and keeps load unitless', async () => {
    const { store } = controller();
    const wrapper = await panel(store);
    expect(wrapper.get('[data-direction="receive"]').text()).toBe('1.0 MiB/s');
    expect(wrapper.get('[data-direction="transmit"]').text()).toBe('256.0 KiB/s');
    expect(wrapper.get('[data-metric="load"] dd').text()).toBe('0.42');
    expect(wrapper.get('[data-metric="load"] dd').text()).not.toContain('%');
  });

  it('emits one Full Monitor action and does not add snapshot requests', async () => {
    const { store, getSnapshot } = controller();
    await flushPromises();
    const wrapper = await panel(store);
    const snapshotCount = getSnapshot.mock.calls.length;
    await wrapper.get('.monitor-open-full').trigger('click');
    expect(wrapper.emitted('full')).toHaveLength(1);
    expect(getSnapshot).toHaveBeenCalledTimes(snapshotCount);
    await wrapper.setProps({ ready: false });
    await wrapper.get('.monitor-open-full').trigger('click');
    expect(wrapper.emitted('full')).toHaveLength(1);
  });

  it('localizes all Quick Monitor labels when the interface language changes', async () => {
    const { store } = controller();
    const wrapper = await panel(store);
    expect(wrapper.text()).toContain('接收');
    locale.value = 'en';
    await flushPromises();
    expect(wrapper.attributes('aria-label')).toBe('Monitor overview');
    expect(wrapper.text()).toContain('Receive');
    expect(wrapper.text()).toContain('Transmit');
    expect(wrapper.text()).not.toContain('接收');
    expect(wrapper.text()).not.toContain('发送');
  });
});
