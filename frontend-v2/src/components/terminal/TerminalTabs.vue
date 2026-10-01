<script setup lang="ts">
import type { TerminalTab } from "../../terminal/controller";
import BaseButton from "../base/BaseButton.vue";
defineProps<{ tabs: TerminalTab[]; activeId: string | null }>();
const emit = defineEmits<{ activate: [id: string, focus: boolean]; close: [id: string] }>();
function navigate(event: KeyboardEvent, id: string, tabs: TerminalTab[]) {
  const current = tabs.findIndex(tab => tab.id === id);
  const next = event.key === 'ArrowRight' ? (current + 1) % tabs.length : event.key === 'ArrowLeft' ? (current - 1 + tabs.length) % tabs.length : event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : -1;
  if (next < 0) return;
  event.preventDefault(); const target = tabs[next]!; emit('activate', target.id, false);
  document.getElementById(`terminal-tab-${target.id}`)?.focus();
}
</script>
<template>
  <div class="terminal-tabs" role="tablist" aria-label="终端标签">
    <div v-for="tab in tabs" :key="tab.id" class="terminal-tab-group">
      <BaseButton :id="`terminal-tab-${tab.id}`" role="tab" :aria-selected="tab.id === activeId" :aria-controls="`terminal-panel-${tab.id}`" :tabindex="tab.id === activeId ? 0 : -1" @click="emit('activate',tab.id, true)" @keydown="navigate($event, tab.id, tabs)">{{ tab.title }}<span v-if="tab.state !== 'running'"> · {{ tab.state === 'opening' ? '启动中' : tab.state === 'closing' ? '关闭中' : '已停止' }}</span></BaseButton>
      <BaseButton class="terminal-tab-close" :aria-label="`关闭 ${tab.title}`" :disabled="tab.state === 'opening' || tab.state === 'closing'" @click="emit('close', tab.id)">×</BaseButton>
    </div>
  </div>
</template>
