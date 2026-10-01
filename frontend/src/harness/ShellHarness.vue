<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppShell from "../app/AppShell.vue";
import { createIpcClient } from "../ipc/client";
import { createShellMock } from "./shell-fixtures";
const params = new URLSearchParams(location.search);
const state = ref<"empty" | "servers">(params.get("state") === "servers" ? "servers" : "empty");
const theme = ref(params.get("theme") === "dark" ? "dark" : "light");
const client = computed(() => createIpcClient(createShellMock(state.value)));
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
</script>

<template>
  <AppShell :key="state" :client="client" read-only />
  <details class="shell-harness-controls">
    <summary>Mock IPC</summary>
    <label>导航状态<select v-model="state"><option value="empty">空列表</option><option value="servers">有服务器</option></select></label>
    <label>验收主题<select v-model="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label>
    <p>仅为 Shell 验收 fixture，不连接服务器或保存资料。</p>
  </details>
</template>

<style scoped>
.shell-harness-controls { position: fixed; right: 12px; bottom: 36px; z-index: 50; max-width: 240px; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: var(--radius); color: var(--color-text-secondary); background: var(--color-surface); font-size: 11px; }
.shell-harness-controls summary { cursor: pointer; }
.shell-harness-controls label { display: grid; gap: 4px; margin: 10px 0; }
.shell-harness-controls select { color: var(--color-text-primary); background: var(--color-surface); }
.shell-harness-controls p { margin: 8px 0 0; }
</style>
