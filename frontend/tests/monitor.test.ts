import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, h } from 'vue';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import type { MonitorHistoryMetric } from '../../contracts/v1/MonitorHistoryMetric';
import type { MonitorSnapshot } from '../../contracts/v1/MonitorSnapshot';
import { createIpcClient } from '../src/ipc/client';
import { createMockIpc, type MockHandlers } from '../src/ipc/mock';
import { createMonitorApi } from '../src/ipc/monitor';
import { createMonitorStore } from '../src/stores/monitor';
import { formatSize, metricValue, percent, rate, sparkline, uptime } from '../src/monitor/view';
import { createMonitorMock, monitorFixture } from '../src/harness/monitor-fixtures';
import { fixtureError } from '../src/harness/server-fixtures';
import MonitorView from '../src/components/monitor/MonitorView.vue';
import MonitorHarness from '../src/harness/MonitorHarness.vue';
const stores: ReturnType<typeof createMonitorStore>[] = []; const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(value => value.unmount()); stores.splice(0).forEach(value => value.dispose()); vi.useRealTimers(); document.body.innerHTML = ''; });
function controller(handlers: MockHandlers = {}) {
  const get = vi.fn(({ connectionId }: { connectionId: string }) => monitorFixture(connectionId));
  const history = vi.fn(({ connectionId, metric }: { connectionId: string; metric: MonitorHistoryMetric }) => ({ connectionId, metric, samples: [{ sampledAtMs: Date.now(), value: 0 }] }));
  const refresh = vi.fn(({ connectionId }: { connectionId: string }) => monitorFixture(connectionId)); const activity = vi.fn();
  const store = createMonitorStore(createMonitorApi(createIpcClient(createMockIpc({ monitor_get_snapshot: get, monitor_get_history: history, monitor_refresh: refresh, workspace_set_activity: activity, ...handlers })))); stores.push(store);
  return { store, get, history, refresh, activity };
}
const view = (store: ReturnType<typeof createMonitorStore>, quick = false) => { const wrapper = mount(MonitorView, { props: { store, quick, ready: true, connectionId: 'conn-a' } }); wrappers.push(wrapper); return wrapper; };
describe('one shared monitor controller', () => {
  it('sends initial home activity once so Rust remains responsible for background scheduling', async () => {
    const { store, get, activity } = controller(); store.activate(null, false); store.activate(null, false); await flushPromises(); expect(activity).toHaveBeenCalledTimes(1); expect(activity).toHaveBeenCalledWith({ activeConnectionId: null, monitorVisible: false }); expect(get).not.toHaveBeenCalled();
  });
  it('polls a shared snapshot once per cycle and batches history at five-second cadence', async () => {
    vi.useFakeTimers(); const { store, get, history } = controller(); store.activate('conn-a', true); await flushPromises();
    const quick = view(store, true); const full = view(store); expect(quick.get('[aria-label="CPU 使用率"] .monitor-value').text()).toBe(full.get('[aria-label="CPU 使用率"] .monitor-value').text());
    expect(get).toHaveBeenCalledTimes(1); expect(history).toHaveBeenCalledTimes(6); await vi.advanceTimersByTimeAsync(4000); expect(get).toHaveBeenCalledTimes(5); expect(history).toHaveBeenCalledTimes(6);
    await vi.advanceTimersByTimeAsync(1000); expect(history).toHaveBeenCalledTimes(12); expect(history).toHaveBeenCalledWith(expect.objectContaining({ fromMs: expect.any(Number), toMs: expect.any(Number), limit: 120 }));
  });
  it('stops reads while hidden, keeps Core background activity, and resumes a single loop', async () => {
    vi.useFakeTimers(); const { store, get, activity } = controller(); store.activate('conn-a', true); await flushPromises(); store.activate('conn-a', false); await flushPromises(); await vi.advanceTimersByTimeAsync(5000);
    expect(get).toHaveBeenCalledTimes(1); expect(activity).toHaveBeenLastCalledWith({ activeConnectionId: 'conn-a', monitorVisible: false });
    store.activate('conn-a', true); await flushPromises(); await vi.advanceTimersByTimeAsync(1000); expect(get).toHaveBeenCalledTimes(3); store.dispose(); await vi.advanceTimersByTimeAsync(5000); expect(get).toHaveBeenCalledTimes(3);
  });
  it('ignores late former-connection responses and clears metrics before switching', async () => {
    let finish!: (value: MonitorSnapshot) => void; const { store } = controller({ monitor_get_snapshot: ({ connectionId }) => connectionId === 'old' ? new Promise(resolve => { finish = resolve; }) : monitorFixture(connectionId) });
    store.activate('old', true); store.activate('conn-a', true); await flushPromises(); finish(monitorFixture('old')); await flushPromises(); expect(store.snapshot.value?.connectionId).toBe('conn-a');
  });
  it('serializes slow activity updates so the latest workspace wins', async () => {
    let finish!: () => void; const calls: string[] = []; const { store } = controller({ workspace_set_activity: ({ activeConnectionId }) => { calls.push(activeConnectionId!); if (activeConnectionId === 'old') return new Promise(resolve => { finish = resolve; }); } });
    store.activate('old', true); await flushPromises(); store.activate('conn-a', true); await flushPromises(); expect(calls).toEqual(['old']); finish(); await flushPromises(); expect(calls).toEqual(['old', 'conn-a']);
  });
  it('avoids poll/manual-refresh overlap and applies the authoritative refreshed snapshot', async () => {
    let finish!: (value: MonitorSnapshot) => void; const refresh = vi.fn(() => new Promise<MonitorSnapshot>(resolve => { finish = resolve; })); const { store } = controller({ monitor_refresh: refresh });
    store.activate('conn-a', true); await flushPromises(); const pending = store.refresh(); expect(await store.refresh()).toBe(false); expect(refresh).toHaveBeenCalledTimes(1); expect(store.refreshing.value).toBe(true);
    finish({ ...monitorFixture('conn-a'), cpu: { ...monitorFixture('conn-a').cpu, usagePercent: 55.1 } }); await pending; expect(store.snapshot.value?.cpu.usagePercent).toBe(55.1); expect(store.refreshing.value).toBe(false);
  });
  it('preserves last data but identifies IPC failure as stale, then recovers', async () => {
    let failure = false; const store = createMonitorStore(createMonitorApi(createIpcClient(createMonitorMock(() => 'ok', { failure: () => failure })))); stores.push(store);
    store.activate('conn-a', true); await flushPromises(); const full = view(store); failure = true; await store.refresh(); await flushPromises(); expect(full.text()).toContain('并非实时数据'); expect(full.get('[aria-label="CPU 使用率"]').attributes('data-quality')).toBe('stale'); expect(full.text()).toContain('24.5%');
    failure = false; await store.refresh(); await flushPromises(); expect(full.findAll('[role="alert"]').filter(alert => alert.isVisible())).toHaveLength(0); expect(full.get('[aria-label="CPU 使用率"]').attributes('data-quality')).toBe('ok');
  });
  it('skips unsupported histories rather than fabricating empty metrics as zero', async () => {
    const { store, history } = controller({ monitor_get_snapshot: ({ connectionId }) => monitorFixture(connectionId, 'unsupported') }); store.activate('conn-a', true); await flushPromises(); expect(history).not.toHaveBeenCalled(); const full = view(store); expect(full.get('[aria-label="CPU 使用率"] .monitor-value').text()).toBe('不支持'); expect(full.findAll('svg.monitor-chart')).toHaveLength(0);
  });
  it('bounds history and reports every metric failure including network transmit', async () => {
    let failure = false; const { store } = controller({ monitor_get_history: ({ connectionId, metric }) => { if (failure && metric === 'networkTransmitRate') throw fixtureError('MONITOR_TIMEOUT', 'errors.monitorTimeout'); return { connectionId, metric, samples: Array.from({ length: 900 }, (_, index) => ({ sampledAtMs: index, value: index })) }; } });
    store.activate('conn-a', true); await flushPromises(); expect(store.histories.value.cpuUsage).toHaveLength(120); const full = view(store); failure = true; await store.refresh(); await flushPromises(); expect(store.histories.value.networkTransmitRate).toHaveLength(120); expect(full.text()).toContain('发送速率历史读取失败'); expect(full.text()).not.toContain('Fixture debug');
  });
  it('retains the final snapshot on disconnect and renders it explicitly as previous sampling', async () => {
    const { store } = controller(); store.activate('conn-a', true); await flushPromises(); store.activate(null, false); const full = view(store); await full.setProps({ ready: false }); expect(full.text()).toContain('保留最后采样'); expect(full.get('[aria-label="CPU 使用率"]').attributes('data-quality')).toBe('stale');
  });
});
describe('truthful monitor presentation', () => {
  it.each(['ok', 'warmingUp', 'stale', 'unsupported', 'error'] as const)('maps %s identically in Quick and Full Monitor', async status => {
    const { store } = controller({ monitor_get_snapshot: ({ connectionId }) => monitorFixture(connectionId, status) }); store.activate('conn-a', true); await flushPromises(); const quick = view(store, true); const full = view(store);
    expect(quick.get('[aria-label="CPU 使用率"] .monitor-value').text()).toBe(full.get('[aria-label="CPU 使用率"] .monitor-value').text()); expect(full.get('[aria-label="CPU 使用率"]').attributes('data-quality')).toBe(status);
    if (status === 'unsupported' || status === 'warmingUp' || status === 'error') expect(full.get('[aria-label="CPU 使用率"] .monitor-value').text()).not.toContain('0%');
  });
  it('formats valid zero, missing, huge metadata, uptime and bounded history without NaN', () => {
    expect(formatSize('4294967296')).toBe('4.0 GiB'); expect(percent(0)).toBe('0.0%'); expect(rate(0)).toBe('0 B/s'); expect(percent(null)).toBe('—'); expect(uptime('90061')).toBe('1 天 1 小时'); expect(metricValue('99%', monitorFixture('a', 'unsupported').cpu.quality)).toBe('不支持');
    expect(sparkline([])).toBe(''); expect(sparkline([{ sampledAtMs: 0, value: NaN }])).toBe(''); expect(sparkline([{ sampledAtMs: 0, value: 0 }])).not.toContain('NaN');
  });
  it('updates charts and cards without rerendering the workspace xterm host', async () => {
    vi.useFakeTimers(); let renders = 0; const host = defineComponent({ props: ['id', 'controller', 'active'], setup: props => () => { renders++; return h('div', { 'data-terminal-id': props.id }); } });
    const wrapper = mount(MonitorHarness, { attachTo: document.body, global: { stubs: { XtermHost: host, Teleport: true } } }); wrappers.push(wrapper); await flushPromises(); await wrapper.get('[aria-label="查看 Web-01"]').trigger('click'); await wrapper.findAll('button').find(value => value.text() === '连接服务器')!.trigger('click'); await flushPromises();
    await wrapper.findAll('button').find(value => value.text() === '监控')!.trigger('click'); await flushPromises();
    // Isolate monitor updates from the existing 1.4-second connection snapshot polling.
    const baseline = renders; expect(baseline).toBeGreaterThan(0);
    for (let index = 0; index < 6; index++) { await wrapper.findAll('button').find(value => value.text() === '立即刷新')!.trigger('click'); await flushPromises(); }
    expect(renders).toBe(baseline); expect(wrapper.text()).toContain('24.5%');
  });
});
