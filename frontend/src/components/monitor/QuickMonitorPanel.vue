<script setup lang="ts">
import { computed } from "vue";
import type { MonitorMetricQuality } from "../../../../contracts/v1/MonitorMetricQuality";
import type { MonitorStore } from "../../stores/monitor";
import { locale, messages } from "../../i18n/locale";
import { monitorMessages } from "../../i18n/monitor";
import { effectiveStatus, loadValue, metricValue, percent, qualityLabel, rate } from "../../monitor/view";
import { quickCollectionState } from "../../monitor/quick";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import MonitorChart from "./MonitorChart.vue";

const t = messages(monitorMessages);
const props = defineProps<{ store: MonitorStore; connectionId: string; ready: boolean }>();
const emit = defineEmits<{ full: [] }>();

const snapshot = computed(() => {
  const value = props.store.snapshot.value;
  return value?.connectionId === props.connectionId ? value : null;
});
const fetchFailed = computed(() => !!props.store.error.value);
const collectionState = computed(() => {
  const data = snapshot.value;
  const statuses = data ? [
    data.cpu.quality.status,
    data.memory.quality.status,
    data.disk.quality.status,
    data.network.quality.status,
    data.load.quality.status,
  ] : [];
  return quickCollectionState(props.ready, !!data, statuses, fetchFailed.value);
});
const collectionStateKey = {
  waiting: 'quickStateWaiting',
  ok: 'quickStateOk',
  partial: 'quickStatePartial',
  stale: 'quickStateStale',
  unavailable: 'quickStateUnavailable',
  disconnected: 'quickStateDisconnected',
} as const;

const statusFor = (quality?: MonitorMetricQuality) =>
  effectiveStatus(quality, fetchFailed.value || !props.ready);
const cpuQuality = computed(() => snapshot.value?.cpu.quality);
const cpuStatus = computed(() => statusFor(cpuQuality.value));
const cpuText = computed(() => metricValue(percent(snapshot.value?.cpu.usagePercent), cpuQuality.value));
const cpuHistory = computed(() => snapshot.value ? props.store.histories.value.cpuUsage ?? [] : []);
const cpuHistoryFailed = computed(() => !!props.store.historyErrors.value.cpuUsage);
const validCpuHistory = computed(() => cpuHistory.value.filter(sample => Number.isFinite(sample.value)));
const showCpuTrend = computed(() =>
  !!snapshot.value && cpuStatus.value === 'ok' && !fetchFailed.value &&
  !cpuHistoryFailed.value && validCpuHistory.value.length >= 2,
);
const cpuTrendText = computed(() =>
  cpuHistoryFailed.value || cpuStatus.value === 'stale' || cpuStatus.value === 'unsupported' || cpuStatus.value === 'error'
    ? t('quickTrendUnavailable')
    : t('quickTrendWaiting'),
);

const memoryStatus = computed(() => statusFor(snapshot.value?.memory.quality));
const memoryText = computed(() => metricValue(percent(snapshot.value?.memory.usedPercent), snapshot.value?.memory.quality));
const diskStatus = computed(() => statusFor(snapshot.value?.disk.quality));
const diskText = computed(() => metricValue(percent(snapshot.value?.disk.usedPercent), snapshot.value?.disk.quality));
const networkStatus = computed(() => statusFor(snapshot.value?.network.quality));
const receiveText = computed(() => metricValue(rate(snapshot.value?.network.receivedBytesPerSecond), snapshot.value?.network.quality));
const transmitText = computed(() => metricValue(rate(snapshot.value?.network.transmittedBytesPerSecond), snapshot.value?.network.quality));
const loadStatus = computed(() => statusFor(snapshot.value?.load.quality));
const loadText = computed(() => metricValue(loadValue(snapshot.value?.load.oneMinute), snapshot.value?.load.quality));

const latestSample = computed(() => {
  const data = snapshot.value;
  if (!data) return null;
  const times = [data.cpu, data.memory, data.disk, data.network, data.load]
    .map(metric => metric.quality.sampledAtMs)
    .filter((value): value is number => typeof value === 'number' && Number.isFinite(value) && value > 0);
  return times.length ? Math.max(...times) : null;
});
const timeText = computed(() => {
  if (latestSample.value === null) return t('awaitingSamples');
  const time = new Intl.DateTimeFormat(locale.value === 'en' ? 'en-US' : 'zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(latestSample.value));
  return props.ready ? t('quickUpdatedAt', { time }) : t('quickLastSampleAt', { time });
});
</script>

<template>
  <section class="quick-monitor" :data-collection-state="collectionState" :aria-label="t('quickMonitorTitle')">
    <div class="quick-monitor-layout">
      <header class="quick-monitor-header">
        <div class="quick-monitor-heading-row">
          <h2>{{ t('quickMonitorTitle') }}</h2>
          <span class="quick-monitor-state" :data-state="collectionState" role="status">
            <span class="quick-monitor-state-dot" aria-hidden="true" />
            {{ t(collectionStateKey[collectionState]) }}
          </span>
        </div>
        <p class="quick-monitor-time">{{ timeText }}</p>
      </header>

      <article class="quick-monitor-hero" :aria-label="t('cpuUsage')" :data-quality="cpuStatus">
        <div class="quick-monitor-hero-head">
          <span>{{ t('cpuUsage') }}</span>
          <small v-if="cpuStatus !== 'ok'">{{ qualityLabel(cpuStatus) }}</small>
        </div>
        <strong class="quick-monitor-hero-value">{{ cpuText }}</strong>
        <MonitorChart
          v-if="showCpuTrend"
          :samples="validCpuHistory"
          :label="t('quickCpuTrendLabel')"
        />
        <p v-else class="quick-monitor-trend-placeholder">{{ cpuTrendText }}</p>
      </article>

      <div class="quick-monitor-secondary">
        <div class="quick-monitor-metric" data-metric="memory" :data-quality="memoryStatus">
          <dl class="quick-monitor-metric-main">
            <dt class="quick-monitor-metric-label">{{ t('memory') }}</dt>
            <dd class="quick-monitor-metric-value">{{ memoryText }}</dd>
          </dl>
          <span v-if="memoryStatus !== 'ok'" class="quick-monitor-metric-quality">{{ qualityLabel(memoryStatus) }}</span>
        </div>

        <div class="quick-monitor-metric" data-metric="disk" :data-quality="diskStatus">
          <dl class="quick-monitor-metric-main">
            <dt class="quick-monitor-metric-label">{{ t('rootFilesystem') }}</dt>
            <dd class="quick-monitor-metric-value">{{ diskText }}</dd>
          </dl>
          <span v-if="diskStatus !== 'ok'" class="quick-monitor-metric-quality">{{ qualityLabel(diskStatus) }}</span>
        </div>

        <div class="quick-monitor-metric quick-monitor-metric--network" data-metric="network" :data-quality="networkStatus">
          <div class="quick-monitor-metric-main">
            <span class="quick-monitor-metric-label">{{ t('networkRate') }}</span>
            <span v-if="networkStatus !== 'ok'" class="quick-monitor-metric-quality">{{ qualityLabel(networkStatus) }}</span>
          </div>
          <dl class="quick-monitor-network-values">
            <div>
              <dt>{{ t('quickReceive') }}</dt>
              <dd data-direction="receive">{{ receiveText }}</dd>
            </div>
            <div>
              <dt>{{ t('quickTransmit') }}</dt>
              <dd data-direction="transmit">{{ transmitText }}</dd>
            </div>
          </dl>
        </div>

        <div class="quick-monitor-metric" data-metric="load" :data-quality="loadStatus">
          <dl class="quick-monitor-metric-main">
            <dt class="quick-monitor-metric-label">{{ t('systemLoad') }}</dt>
            <dd class="quick-monitor-metric-value">{{ loadText }}</dd>
          </dl>
          <span v-if="loadStatus !== 'ok'" class="quick-monitor-metric-quality">{{ qualityLabel(loadStatus) }}</span>
        </div>
      </div>

      <div class="quick-monitor-footer">
        <BaseButton class="monitor-open-full" variant="softPrimary" size="md" block :disabled="!ready" @click="emit('full')">
          <BaseIcon name="monitor" />{{ t('openFullMonitor') }}
        </BaseButton>
      </div>
    </div>
  </section>
</template>
