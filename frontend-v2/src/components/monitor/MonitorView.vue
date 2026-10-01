<script setup lang="ts">
import { computed } from "vue";
import type { MonitorStore } from "../../stores/monitor";
import type { MonitorHistoryMetric } from "../../../../contracts/v1/MonitorHistoryMetric";
import { effectiveStatus, formatSize, metricValue, percent, rate, loadValue, qualityText, uptime } from "../../monitor/view";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import MonitorChart from "./MonitorChart.vue";
const props = defineProps<{ store: MonitorStore; connectionId: string; quick?: boolean; ready: boolean }>();
const emit = defineEmits<{ full: [] }>();
const snapshot = computed(() => props.store.snapshot.value?.connectionId === props.connectionId ? props.store.snapshot.value : null);
const samples = (name: MonitorHistoryMetric) => snapshot.value ? props.store.histories.value[name] ?? [] : [];
const cards = computed(() => {
  const data = snapshot.value;
  return [
    { key: 'cpu', label: 'CPU 使用率', quality: data?.cpu.quality, value: percent(data?.cpu.usagePercent), detail: `逻辑核心 ${data?.cpu.logicalCores ?? '—'}`, metric: 'cpuUsage' as const },
    { key: 'memory', label: '内存', quality: data?.memory.quality, value: percent(data?.memory.usedPercent), detail: `${formatSize(data?.memory.usedBytes ?? null)} / ${formatSize(data?.memory.totalBytes ?? null)} · 可用 ${formatSize(data?.memory.availableBytes ?? null)}`, metric: 'memoryUsage' as const },
    { key: 'disk', label: '主文件系统', quality: data?.disk.quality, value: percent(data?.disk.usedPercent), detail: `${formatSize(data?.disk.usedBytes ?? null)} / ${formatSize(data?.disk.totalBytes ?? null)} · ${data?.disk.mount ?? '—'}`, metric: 'diskUsage' as const },
    { key: 'network', label: '网络速率', quality: data?.network.quality, value: `↓ ${rate(data?.network.receivedBytesPerSecond)} · ↑ ${rate(data?.network.transmittedBytesPerSecond)}`, detail: data?.network.interfaces.length ? `接口 ${data.network.interfaces.map(value => value.name).join(' · ')} · 不含环回` : '服务器视角；不含环回接口', metric: 'networkReceiveRate' as const },
    { key: 'load', label: '系统负载', quality: data?.load.quality, value: loadValue(data?.load.oneMinute), detail: `${loadValue(data?.load.oneMinute)} / ${loadValue(data?.load.fiveMinutes)} / ${loadValue(data?.load.fifteenMinutes)} · 1 / 5 / 15 分钟`, metric: 'loadOneMinute' as const },
  ];
});
const latest = computed(() => snapshot.value ? Math.max(...cards.value.map(value => value.quality?.sampledAtMs ?? 0), snapshot.value.system.quality.sampledAtMs ?? 0, snapshot.value.uptime.quality.sampledAtMs ?? 0) : 0);
</script>
<template>
  <section :class="quick ? 'quick-monitor' : 'full-monitor'" :aria-label="quick ? 'Quick Monitor' : 'Full Monitor'">
    <header><div><h2>{{ quick ? 'Quick Monitor' : '服务器概览' }}</h2><p v-if="!quick">指标来自 Core 固定采集脚本；图表显示最近 2 分钟趋势，各线独立缩放。</p></div><BaseButton v-if="!quick" :disabled="!ready || store.pending.value" @click="store.refresh()">{{ store.refreshing.value ? '正在刷新…' : '立即刷新' }}</BaseButton></header>
    <p v-if="!ready" class="monitor-notice" role="status">{{ snapshot ? '连接已结束，保留最后采样。' : '连接已结束，无可用采样。' }}</p>
    <p v-if="store.error.value" class="monitor-notice" role="alert">{{ presentError(store.error.value).message }}<span v-if="snapshot"> · 以下为上次快照，并非实时数据。</span></p>
    <p v-if="store.activityError.value" class="monitor-notice" role="alert">监控活动状态同步失败：{{ presentError(store.activityError.value).message }}</p>
    <p v-for="(failure, metric) in store.historyErrors.value" v-show="failure" :key="metric" class="monitor-notice" role="alert">{{ {cpuUsage: 'CPU', memoryUsage: '内存', diskUsage: '磁盘', networkReceiveRate: '接收速率', networkTransmitRate: '发送速率', loadOneMinute: '负载'}[metric] }}历史读取失败，图表可能过期：{{ failure && presentError(failure).message }}</p>
    <p class="monitor-updated" role="status">{{ latest ? `最近采集 ${new Date(latest).toLocaleTimeString()}` : '等待采样' }}</p>
    <div class="monitor-grid"><article v-for="card in cards" :key="card.key" :aria-label="card.label" class="monitor-card" :data-quality="effectiveStatus(card.quality, !!store.error.value || !ready)">
      <div class="monitor-card-heading"><h3>{{ card.label }}</h3><span>{{ qualityText[effectiveStatus(card.quality, !!store.error.value || !ready)] }}</span></div>
      <strong class="monitor-value">{{ metricValue(card.value, card.quality) }}</strong>
      <p v-if="!quick && card.quality?.status !== 'unsupported' && card.quality?.status !== 'error'">{{ card.detail }}</p>
      <MonitorChart v-if="(!quick || card.key === 'cpu') && card.quality?.status !== 'unsupported' && card.quality?.status !== 'error'" :samples="samples(card.metric)" :secondary="card.key === 'network' ? samples('networkTransmitRate') : undefined" :label="`${card.label} 最近 2 分钟历史`" />
    </article><article v-if="!quick" class="monitor-card monitor-system" aria-label="系统信息" :data-quality="effectiveStatus(snapshot?.system.quality, !!store.error.value || !ready)"><div class="monitor-card-heading"><h3>系统信息</h3><span>{{ qualityText[effectiveStatus(snapshot?.system.quality, !!store.error.value || !ready)] }}</span></div>
      <dl><dt>主机</dt><dd>{{ metricValue(snapshot?.system.hostname ?? '—', snapshot?.system.quality) }}</dd><dt>系统</dt><dd>{{ metricValue(snapshot?.system.os ?? '—', snapshot?.system.quality) }}</dd><dt>内核</dt><dd>{{ metricValue(snapshot?.system.kernel ?? '—', snapshot?.system.quality) }}</dd><dt>架构</dt><dd>{{ metricValue(snapshot?.system.architecture ?? '—', snapshot?.system.quality) }}</dd><dt>运行时间</dt><dd>{{ metricValue(uptime(snapshot?.uptime.seconds), snapshot?.uptime.quality) }} · {{ qualityText[effectiveStatus(snapshot?.uptime.quality, !!store.error.value || !ready)] }}</dd></dl>
    </article></div>
    <BaseButton v-if="quick" class="monitor-open-full" :disabled="!ready" @click="emit('full')">查看完整监控</BaseButton>
  </section>
</template>
