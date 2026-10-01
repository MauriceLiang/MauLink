<script setup lang="ts">
import { ref, watch } from "vue";
import AppShell from "../app/AppShell.vue";
import { createIpcClient } from "../ipc/client";
import { createConnectionMock, type ConnectionScenario } from "./connection-fixtures";
const params = new URLSearchParams(location.search);
const native = params.get('transport') === 'native';
const scenario = ref<ConnectionScenario>('unknown');
const theme = ref(params.get('theme') === 'dark' ? 'dark' : 'light');
const slow = ref(false);
const mock = createConnectionMock(() => scenario.value, () => slow.value ? 10000 : 0);
const client = native ? createIpcClient() : createIpcClient(mock.transport);
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
</script>
<template>
  <AppShell :client="client" />
  <details class="connection-harness"><summary>{{ native ? 'Native SSH 验收' : 'Mock SSH 验收' }}</summary>
    <label>主题<select v-model="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label>
    <label v-if="!native">下一次连接场景<select v-model="scenario"><option value="unknown">首次 Host Key</option><option value="changed">Host Key Changed</option><option value="password">密码挑战</option><option value="passphrase">私钥口令挑战</option><option value="refused">连接拒绝</option><option value="timeout">连接超时</option><option value="proxy">代理连接失败</option><option value="jump">跳板连接失败</option><option value="expired">挑战过期</option></select></label>
    <label v-if="!native"><input v-model="slow" type="checkbox" />延迟挑战回应 10 秒</label>
    <p>{{ native ? '真实 Core / SSH，只操作隔离验收资料。' : '内存模拟，wrong 演示认证失败；其他一次性输入演示成功，不访问真实服务器。' }}</p>
  </details>
</template>
<style scoped>
.connection-harness { position: fixed; right: 12px; bottom: 144px; z-index: 50; max-width: 250px; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); color: var(--color-text-secondary); background: var(--color-surface); font-size: 11px; }
.connection-harness label { display: grid; gap: 4px; margin: 10px 0; }
.connection-harness select { color: var(--color-text-primary); background: var(--color-surface); }
</style>
