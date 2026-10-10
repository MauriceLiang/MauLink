<script setup lang="ts">
import BaseAlert from "../base/BaseAlert.vue";
import { messages } from "../../i18n/locale";
import { monitorMessages } from "../../i18n/monitor";
import { computed } from "vue";
import type { MonitorStore } from "../../stores/monitor";
import type { MonitorHistoryMetric } from "../../../../contracts/v1/MonitorHistoryMetric";
import { effectiveStatus, formatSize, metricValue, percent, rate, loadValue, qualityLabel, uptime } from "../../monitor/view";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import MonitorChart from "./MonitorChart.vue";
import QuickMonitorPanel from "./QuickMonitorPanel.vue";
const t = messages(monitorMessages);
const props = defineProps<{ store: MonitorStore; connectionId: string; quick?: boolean; ready: boolean }>();
const emit = defineEmits<{ full: [] }>();
const snapshot = computed(() => props.store.snapshot.value?.connectionId === props.connectionId ? props.store.snapshot.value : null);
const samples = (name: MonitorHistoryMetric) => snapshot.value ? props.store.histories.value[name] ?? [] : [];
const cards = computed(() => {
  const data = snapshot.value;
  return [
    { key: 'cpu', label: t('cpuUsage'), quality: data?.cpu.quality, value: percent(data?.cpu.usagePercent), detail: t('cores', {count: data?.cpu.logicalCores ?? '—'}), metric: 'cpuUsage' as const },
    { key: 'memory', label: t('memory'), quality: data?.memory.quality, value: percent(data?.memory.usedPercent), detail: `${formatSize(data?.memory.usedBytes ?? null)} / ${formatSize(data?.memory.totalBytes ?? null)} · ${t('available', {value: formatSize(data?.memory.availableBytes ?? null)})}`, metric: 'memoryUsage' as const },
    { key: 'disk', label: t('rootFilesystem'), quality: data?.disk.quality, value: percent(data?.disk.usedPercent), detail: `${formatSize(data?.disk.usedBytes ?? null)} / ${formatSize(data?.disk.totalBytes ?? null)} · ${data?.disk.mount ?? '—'}`, metric: 'diskUsage' as const },
    { key: 'network', label: t('networkRate'), quality: data?.network.quality, value: `↓ ${rate(data?.network.receivedBytesPerSecond)} · ↑ ${rate(data?.network.transmittedBytesPerSecond)}`, detail: data?.network.interfaces.length ? t('interfaces', {names: data.network.interfaces.map(value => value.name).join(' · ')}) : t('networkNote'), metric: 'networkReceiveRate' as const },
    { key: 'load', label: t('systemLoad'), quality: data?.load.quality, value: loadValue(data?.load.oneMinute), detail: `${loadValue(data?.load.oneMinute)} / ${loadValue(data?.load.fiveMinutes)} / ${loadValue(data?.load.fifteenMinutes)} · ${t('minutes')}`, metric: 'loadOneMinute' as const },
  ];
});
const latest = computed(() => snapshot.value ? Math.max(...cards.value.map(value => value.quality?.sampledAtMs ?? 0), snapshot.value.system.quality.sampledAtMs ?? 0, snapshot.value.uptime.quality.sampledAtMs ?? 0) : 0);
</script>
<template>
  <QuickMonitorPanel v-if="quick" :store="store" :connection-id="connectionId" :ready="ready" @full="emit('full')" />
  <section v-else class="full-monitor" :aria-label="t('fullMonitorTitle')">
    <header><div><h2>{{ t('serverOverview') }}</h2><p>{{ t('historyNote') }}</p></div><BaseButton :disabled="!ready || store.pending.value" :loading="store.refreshing.value" @click="store.refresh()">{{ t('refreshNow') }}</BaseButton></header>
    <p v-if="!ready" class="monitor-notice" role="status">{{ snapshot ? t('connectionEndedLastSampleRetained') : t('connectionEndedNoSamplesAvailable') }}</p>
    <BaseAlert v-if="store.error.value" class="monitor-notice" >{{ presentError(store.error.value).message }}<span v-if="snapshot"> {{ t('snapshotNote') }}</span></BaseAlert>
    <BaseAlert v-if="store.activityError.value" class="monitor-notice" >{{ t('syncFailed') }}{{ presentError(store.activityError.value).message }}</BaseAlert>
    <BaseAlert v-for="(failure, metric) in store.historyErrors.value" v-show="failure" :key="metric" class="monitor-notice" >{{ {cpuUsage: 'CPU', memoryUsage: t('memory'), diskUsage: t('disk'), networkReceiveRate: t('receiveRate'), networkTransmitRate: t('transmitRate'), loadOneMinute: t('load')}[metric] }}{{ t('historyFailed') }}{{ failure && presentError(failure).message }}</BaseAlert>
    <p class="monitor-updated" role="status">{{ latest ? t('latest', {time: new Date(latest).toLocaleTimeString()}) : t('awaitingSamples') }}</p>
    <div class="monitor-grid"><article v-for="card in cards" :key="card.key" :aria-label="card.label" class="monitor-card" :data-quality="effectiveStatus(card.quality, !!store.error.value || !ready)">
      <div class="monitor-card-heading"><h3>{{ card.label }}</h3><span>{{ qualityLabel(effectiveStatus(card.quality, !!store.error.value || !ready)) }}</span></div>
      <strong class="monitor-value">{{ metricValue(card.value, card.quality) }}</strong>
      <p v-if="card.quality?.status !== 'unsupported' && card.quality?.status !== 'error'">{{ card.detail }}</p>
      <MonitorChart v-if="card.quality?.status !== 'unsupported' && card.quality?.status !== 'error'" :samples="samples(card.metric)" :secondary="card.key === 'network' ? samples('networkTransmitRate') : undefined" :label="t('history', {name: card.label})" />
    </article><article class="monitor-card monitor-system" :aria-label="t('systemInformation')" :data-quality="effectiveStatus(snapshot?.system.quality, !!store.error.value || !ready)"><div class="monitor-card-heading"><h3>{{ t('systemInformation') }}</h3><span>{{ qualityLabel(effectiveStatus(snapshot?.system.quality, !!store.error.value || !ready)) }}</span></div>
      <dl><dt>{{ t('host') }}</dt><dd>{{ metricValue(snapshot?.system.hostname ?? '—', snapshot?.system.quality) }}</dd><dt>{{ t('system') }}</dt><dd>{{ metricValue(snapshot?.system.os ?? '—', snapshot?.system.quality) }}</dd><dt>{{ t('kernel') }}</dt><dd>{{ metricValue(snapshot?.system.kernel ?? '—', snapshot?.system.quality) }}</dd><dt>{{ t('architecture') }}</dt><dd>{{ metricValue(snapshot?.system.architecture ?? '—', snapshot?.system.quality) }}</dd><dt>{{ t('uptime') }}</dt><dd>{{ metricValue(uptime(snapshot?.uptime.seconds), snapshot?.uptime.quality) }} · {{ qualityLabel(effectiveStatus(snapshot?.uptime.quality, !!store.error.value || !ready)) }}</dd></dl>
    </article></div>
  </section>
</template>
