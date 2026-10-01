<script setup lang="ts">
import { computed } from "vue";
import type { MonitorHistorySample } from "../../../../contracts/v1/MonitorHistorySample";
import { sparkline } from "../../monitor/view";
const props = defineProps<{ samples: MonitorHistorySample[]; secondary?: MonitorHistorySample[]; label: string }>();
const path = computed(() => sparkline(props.samples)); const secondaryPath = computed(() => sparkline(props.secondary ?? []));
</script>
<template><svg class="monitor-chart" viewBox="0 0 220 42" preserveAspectRatio="none" role="img" :aria-label="label"><path v-if="path" :d="path" /><path v-if="secondaryPath" :d="secondaryPath" class="is-secondary" /></svg></template>
