<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { TerminalController } from "../../terminal/controller";
const props = defineProps<{ id: string; controller: TerminalController; active: boolean }>();
const host = ref<HTMLElement>();
let observer: ResizeObserver;
let frame = 0;
function schedule() { cancelAnimationFrame(frame); frame = requestAnimationFrame(() => props.controller.fit(props.id)); }
onMounted(() => {
  if (!host.value) return;
  void props.controller.attach(props.id, host.value);
  observer = new ResizeObserver(schedule); observer.observe(host.value);
});
watch(() => props.active, async active => { if (active) { await nextTick(); props.controller.fit(props.id); } });
onBeforeUnmount(() => { observer?.disconnect(); cancelAnimationFrame(frame); props.controller.detach(props.id); });
</script>
<template><div ref="host" class="xterm-host" role="tabpanel" :id="`terminal-panel-${id}`" :aria-labelledby="`terminal-tab-${id}`" /></template>
