<script setup lang="ts">
import { computed } from "vue";
import type { MonitorHistorySample } from "../../../../contracts/v1/MonitorHistorySample";
import { sparkline } from "../../monitor/view";
const props = defineProps<{ samples: MonitorHistorySample[]; secondary?: MonitorHistorySample[]; label: string; domain?: readonly [number, number] }>();
const path = computed(() => sparkline(props.samples, 220, 42, props.domain)); const secondaryPath = computed(() => sparkline(props.secondary ?? [], 220, 42, props.domain));
</script>
<template><svg class="monitor-chart" viewBox="0 0 220 42" preserveAspectRatio="none" role="img" :aria-label="label"><template v-if="domain"><line v-for="y in [21, 40]" :key="y" class="monitor-chart-guide" x1="0" :y1="y" x2="220" :y2="y" /></template><path v-if="path" :d="path" pathLength="1" /><path v-if="secondaryPath" :d="secondaryPath" class="is-secondary" pathLength="1" /></svg></template>
