<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import type { Channel } from "@tauri-apps/api/core";
import type { TerminalChunk } from "../../../contracts/v1/TerminalChunk";
import type { SftpTransferSnapshot } from "../../../contracts/v1/SftpTransferSnapshot";
import type { MonitorQualityStatus } from "../../../contracts/v1/MonitorQualityStatus";
import AppShell from "../app/AppShell.vue";
import { createIpcClient, type IpcTransport } from "../ipc/client";
import { createFilesMock, createFilesWorkspaceMock } from "./files-fixtures";
import { createMonitorMock } from "./monitor-fixtures";
const status = ref<MonitorQualityStatus>('ok'); const failure = ref(false); const historyFailure = ref(false); const theme = ref('light');
const counts = ref({ snapshot: 0, refresh: 0, history: 0, activity: '' });
const files = createFilesMock(); const workspace = createFilesWorkspaceMock(files);
const monitor = createMonitorMock(() => status.value, { failure: () => failure.value, historyFailure: () => historyFailure.value, count: name => { if (name.startsWith('activity:')) counts.value.activity = name; else counts.value[name as 'snapshot' | 'refresh' | 'history']++; } });
const transport: IpcTransport = { invoke: <T,>(command: string, args?: Record<string, unknown>) => command.startsWith('monitor_') || command === 'workspace_set_activity' ? monitor.invoke<T>(command, args) : workspace.invoke<T>(command, args) };
const client = createIpcClient(transport);
const terminalChannelFactory = () => ({ onmessage: (_value: TerminalChunk) => undefined }) as Channel<TerminalChunk>;
const transferChannelFactory = () => ({ onmessage: (_value: SftpTransferSnapshot) => undefined }) as Channel<SftpTransferSnapshot>;
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
onBeforeUnmount(files.dispose);
</script>
<template>
  <AppShell :client="client" :terminal-channel-factory="terminalChannelFactory" :transfer-channel-factory="transferChannelFactory" />
  <details class="monitor-harness" open><summary>Phase 8 · DEV Linux metrics fixture</summary><p>内存 Mock 数据，用于组件验收；终端也为 Mock。</p><label>主题<select v-model="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label><label>质量状态<select v-model="status"><option value="ok">ok</option><option value="warmingUp">warmingUp</option><option value="stale">stale</option><option value="unsupported">unsupported</option><option value="error">error</option></select></label><label><input v-model="failure" type="checkbox" />快照 / 刷新 IPC 失败</label><label><input v-model="historyFailure" type="checkbox" />历史 IPC 失败</label><output aria-label="Monitor IPC 计数">snapshot={{ counts.snapshot }} · refresh={{ counts.refresh }} · history={{ counts.history }} · {{ counts.activity }}</output></details>
</template>
<style scoped>
.monitor-harness { position: fixed; right: 12px; bottom: 34px; z-index: 4; max-width: 260px; border: 1px solid var(--line); border-radius: 8px; padding: 10px; background: var(--panel); color: var(--muted); font-size: 11px; }
.monitor-harness label { display: flex; gap: 8px; margin: 8px 0; }.monitor-harness select { color: var(--ink); background: var(--panel); border: 1px solid var(--line); }.monitor-harness output { display: block; line-height: 1.6; overflow-wrap: anywhere; }
</style>
