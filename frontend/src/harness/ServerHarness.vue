<script setup lang="ts">
import { ref, watch } from "vue";
import AppShell from "../app/AppShell.vue";
import { createIpcClient } from "../ipc/client";
import { createServerMock } from "./server-fixtures";
import { startOccupancyProbe } from "./occupancy-probe";
import { createConnectionApi } from "../ipc/connection";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
const params = new URLSearchParams(location.search);
const native = params.get("transport") === "native";
const theme = ref(params.get("theme") === "dark" ? "dark" : "light");
const failure = ref<"none" | "inUse" | "revision" | "unknown">("none");
const slow = ref(false);
const mock = createServerMock({ state: params.get("state") === "empty" ? "empty" : "servers", failure: () => failure.value, delay: () => slow.value ? 10000 : 0 });
const client = native ? createIpcClient() : createIpcClient(mock);
const connectionId = ref<string | null>(null);
const probeBusy = ref(false);
const probeMessage = ref("");
async function startProbe() {
  if (probeBusy.value || connectionId.value) return;
  probeBusy.value = true;
  try {
    const result = await startOccupancyProbe(client);
    connectionId.value = result?.connectionId ?? null;
    probeMessage.value = result ? "已发起回环 test-mode 连接，请立即验收 Host/Port 修改被阻止，然后取消占用。" : "请先按验收说明创建唯一的 Phase4-占用验收 配置。";
  } catch (reason) { probeMessage.value = presentError(mapError(reason)).message; }
  finally { probeBusy.value = false; }
}
async function cancelProbe() {
  if (probeBusy.value || !connectionId.value) return;
  probeBusy.value = true;
  try {
    await createConnectionApi(client).cancel({ connectionId: connectionId.value });
    connectionId.value = null;
    probeMessage.value = "回环占用已取消，可以恢复编辑并删除临时配置。";
  } catch (reason) { probeMessage.value = presentError(mapError(reason)).message; }
  finally { probeBusy.value = false; }
}
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
</script>

<template>
  <AppShell :client="client" />
  <details class="server-harness-controls"><summary>{{ native ? 'Native Server CRUD 验收' : 'Mock Server CRUD' }}</summary>
    <label>主题<select v-model="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label>
    <template v-if="!native">
      <label>模拟写入错误<select v-model="failure"><option value="none">无</option><option value="inUse">ServerInUse</option><option value="revision">RevisionConflict</option><option value="unknown">未知错误</option></select></label>
      <label><input v-model="slow" type="checkbox" />延迟写入回应 10 秒</label>
      <p>仅操作内存 fixture。私钥选择返回演示 token，不打开真实文件。</p>
    </template>
    <template v-else>
      <p>真实 SQLite / 系统凭据存储 / 文件选择器。仅使用专用验收资料。</p>
      <button :disabled="probeBusy || !!connectionId" @click="startProbe">开始回环占用验收</button>
      <button :disabled="probeBusy || !connectionId" @click="cancelProbe">取消回环占用</button>
      <p role="status">{{ probeMessage }}</p>
    </template>
  </details>
</template>

<style scoped>
.server-harness-controls { position: fixed; right: 12px; bottom: 144px; z-index: 50; max-width: 260px; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); color: var(--color-text-secondary); background: var(--color-surface); font-size: 11px; }
.server-harness-controls summary { cursor: pointer; }
.server-harness-controls label { display: grid; gap: 4px; margin: 10px 0; }
.server-harness-controls select { color: var(--color-text-primary); background: var(--color-surface); }
</style>
