<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import type { Channel } from "@tauri-apps/api/core";
import type { SftpTransferSnapshot } from "../../../contracts/v1/SftpTransferSnapshot";
import type { TerminalChunk } from "../../../contracts/v1/TerminalChunk";
import AppShell from "../app/AppShell.vue";
import FilesPanel from "../components/files/FilesPanel.vue";
import { createIpcClient } from "../ipc/client";
import { createSftpApi } from "../ipc/sftp";
import { createTransferStore } from "../stores/transfers";
import { createFilesMock, createFilesWorkspaceMock, type FileScenario } from "./files-fixtures";
const scenario = ref<FileScenario>('completed'); const theme = ref('light'); const generation = ref(0);
const workspace = new URLSearchParams(location.search).get('workspace') === '1';
const mock = createFilesMock(() => scenario.value); const client = createIpcClient(mock.transport); const api = createSftpApi(client);
const transferChannelFactory = () => ({ onmessage: (_value: SftpTransferSnapshot) => undefined }) as Channel<SftpTransferSnapshot>;
const terminalChannelFactory = () => ({ onmessage: (_value: TerminalChunk) => undefined }) as Channel<TerminalChunk>;
const workspaceClient = workspace ? createIpcClient(createFilesWorkspaceMock(mock)) : null;
const transfers = createTransferStore(api, purpose => client.call('local_file_select', { purpose }), transferChannelFactory);
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
function reset() { mock.reset(); generation.value++; }
onBeforeUnmount(() => { transfers.dispose(); mock.dispose(); });
</script>
<template>
  <AppShell v-if="workspaceClient" :client="workspaceClient" :terminal-channel-factory="terminalChannelFactory" :transfer-channel-factory="transferChannelFactory" />
  <div class="files-harness" :class="{ 'workspace-harness': workspace }"><header><strong>Phase 7 · Browser Harness</strong><span>DEV 内存 SFTP：450 项分页；不访问真实文件，不传输文件字节。</span><label>主题<select v-model="theme"><option value="light">Light</option><option value="dark">Dark</option></select></label><label>传输场景<select v-model="scenario"><option value="completed">完成</option><option value="slow">慢速 / 取消</option><option value="failed">失败 / 清理提示</option><option value="picker-cancel">取消文件选择</option></select></label><button v-if="!workspace" @click="reset">重置目录</button><button @click="mock.expire()">游标过期</button></header><FilesPanel v-if="!workspace" :key="generation" :api="api" :transfers="transfers" connection-id="files-fixture" :ready="true" :visible="true" /></div>
</template>
<style scoped>
.files-harness { height: 100vh; display: grid; grid-template-rows: auto minmax(0, 1fr); background: var(--panel); color: var(--ink); }
.files-harness > header { display: flex; align-items: center; gap: 14px; padding: 14px; border-bottom: 1px solid var(--line); font-size: 12px; flex-wrap: wrap; }
.files-harness > header > span { flex: 1; color: var(--muted); }
.files-harness select, .files-harness > header button { border: 1px solid var(--line); border-radius: 6px; padding: 6px; color: var(--ink); background: var(--panel); }
.workspace-harness > header { padding: 8px; gap: 8px; }
.workspace-harness > header > span { display: none; }
.workspace-harness > header > strong { width: 100%; }
.workspace-harness { position: fixed; right: 12px; bottom: 34px; z-index: 4; width: 250px; height: auto; border: 1px solid var(--line); border-radius: 8px; }
</style>
