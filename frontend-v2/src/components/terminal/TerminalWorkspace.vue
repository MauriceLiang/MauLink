<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import MonitorView from "../monitor/MonitorView.vue";
import type { MonitorStore } from "../../stores/monitor";
import FilesPanel from "../files/FilesPanel.vue";
import type { createSftpApi } from "../../ipc/sftp";
import { transferFinished, type TransferStore } from "../../stores/transfers";
import { presentError } from "../../errors/presenter";
import type { AppError } from "../../../../contracts/v1/AppError";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { ConnectionSnapshot } from "../../../../contracts/v1/ConnectionSnapshot";
import type { TerminalController } from "../../terminal/controller";
import type { TerminalPreferences } from "../../terminal/preferences";
import { connectionStateText } from "../../i18n/connections";
import BaseButton from "../base/BaseButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import TerminalTabs from "./TerminalTabs.vue";
import XtermHost from "./XtermHost.vue";
import TerminalSettingsDialog from "../../dialogs/TerminalSettingsDialog.vue";
const props = defineProps<{ server: ServerProfile; snapshot: ConnectionSnapshot; controller: TerminalController; sftp: ReturnType<typeof createSftpApi>; transfers: TransferStore; monitor: MonitorStore; preferences: TerminalPreferences; visible: boolean; busy: boolean; error?: AppError | null }>();
const emit = defineEmits<{ home: []; disconnect: [stopActiveTransfers: boolean]; focusMode: [enabled: boolean]; view: [pane: 'terminal' | 'files' | 'monitor'] }>();
const root = ref<HTMLElement | null>(null);
const activeId = ref<string | null>(null);
const settingsOpen = ref(false); const disconnectOpen = ref(false); const focused = ref(false);
const tabs = computed(() => props.controller.tabs.value.filter(tab => tab.connectionId === props.snapshot.connectionId));
const active = computed(() => tabs.value.find(tab => tab.id === activeId.value));
const ready = computed(() => props.snapshot.state === 'ready');
const pane = ref<'terminal' | 'files' | 'monitor'>('terminal'); const filesVisited = ref(false); const stopTransfers = ref(false);
const activeTransfers = computed(() => props.transfers.snapshots.value.filter(task => task.connectionId === props.snapshot.connectionId && !transferFinished(task)).length);
const requireStopConfirmation = computed(() => activeTransfers.value > 0 || props.error?.messageKey === 'errors.activeTransfersRequireConfirmation');
async function view(value: 'terminal' | 'files' | 'monitor') { pane.value = value; emit('view', value); if (value !== 'terminal') { if (value === 'files') filesVisited.value = true; focused.value = false; emit('focusMode', false); } else { await nextTick(); if (activeId.value) props.controller.focus(activeId.value); } }
let initialized = false;
async function create() {
  if (!ready.value || props.busy) return;
  activeId.value = props.controller.create(props.snapshot.connectionId);
  await nextTick(); props.controller.focus(activeId.value);
}
watch(() => props.visible, async visible => {
  if (!visible) { focused.value = false; emit('focusMode', false); return; }
  if (!initialized) { initialized = true; if (!props.preferences.record.value) await props.preferences.load(); await create(); }
  else { await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
}, { immediate: true });
watch(ready, value => { if (!value) { void props.controller.refreshConnection(props.snapshot.connectionId); focused.value = false; emit('focusMode', false); } });
async function activate(id: string, focus = true) { activeId.value = id; await nextTick(); if (focus) props.controller.focus(id); }
async function close(id: string) { if (await props.controller.close(id)) { if (activeId.value === id) activeId.value = tabs.value.at(-1)?.id ?? null; await nextTick(); if (activeId.value) props.controller.focus(activeId.value); else root.value?.querySelector<HTMLButtonElement>('.terminal-new')?.focus(); } }
async function toggleFocus() { focused.value = !focused.value; emit('focusMode', focused.value); await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
async function closeSettings() { settingsOpen.value = false; await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
function disconnect() { stopTransfers.value = false; if (requireStopConfirmation.value || (props.preferences.record.value?.value.confirmBeforeDisconnect ?? true)) disconnectOpen.value = true; else emit('disconnect', false); }
watch(() => props.busy, busy => { if (!busy && !ready.value) disconnectOpen.value = false; });
</script>
<template>
  <section ref="root" class="terminal-workspace" :class="{ 'is-focused': focused, 'is-files': pane === 'files', 'is-monitor': pane === 'monitor' }" :aria-label="`${server.name} 工作区`">
    <header class="workspace-heading"><div><h1>{{ server.name }}</h1><span>{{ server.username }}@{{ server.host }}:{{ server.port }}</span></div><span class="workspace-connection-state">{{ connectionStateText(snapshot.state) }}</span><BaseButton class="workspace-view" :aria-pressed="pane === 'terminal'" @click="view('terminal')">终端</BaseButton><BaseButton class="workspace-view" :aria-pressed="pane === 'files'" @click="view('files')">文件</BaseButton><BaseButton class="workspace-view" :aria-pressed="pane === 'monitor'" @click="view('monitor')">监控</BaseButton><BaseButton @click="emit('home')">返回服务器</BaseButton><BaseButton :disabled="!ready || busy || transfers.starting.value[snapshot.connectionId]" @click="disconnect">断开连接</BaseButton></header>
    <div v-show="pane === 'terminal'" class="terminal-toolbar"><TerminalTabs :tabs="tabs" :active-id="activeId" @activate="activate" @close="close" /><BaseButton class="terminal-new" :disabled="!ready || busy" @click="create">新建终端</BaseButton><BaseButton :disabled="!active" :aria-pressed="focused" @click="toggleFocus">{{ focused ? '退出专注' : '专注' }}</BaseButton><BaseButton @click="settingsOpen = true">终端设置</BaseButton><BaseButton :disabled="!active" @click="activeId && controller.clear(activeId)">清屏</BaseButton></div>
    <div v-show="pane === 'terminal'" class="terminal-main"><div class="terminal-stack"><XtermHost v-for="tab in tabs" :key="tab.id" v-show="tab.id === activeId" :id="tab.id" :controller="controller" :active="visible && pane === 'terminal' && tab.id === activeId" /><p v-if="!tabs.length" class="terminal-empty">{{ ready ? '点击“新建终端”打开 Shell。' : 'SSH 连接已结束。' }}</p></div><MonitorView :store="monitor" :connection-id="snapshot.connectionId" :ready="ready" quick @full="view('monitor')" /></div>
    <footer v-show="pane === 'terminal'" class="terminal-footer" role="status"><span>{{ active ? active.state === 'running' ? active.inputPaused ? '输入已暂停' : 'Shell 就绪' : '会话已停止' : '等待 Shell' }}</span><span>{{ active?.columns ?? '—' }} × {{ active?.rows ?? '—' }}</span><span>{{ tabs.length }} 个终端</span></footer>
    <MonitorView v-show="pane === 'monitor'" :store="monitor" :connection-id="snapshot.connectionId" :ready="ready" />
    <FilesPanel v-if="filesVisited" v-show="pane === 'files'" :api="sftp" :transfers="transfers" :connection-id="snapshot.connectionId" :visible="visible && pane === 'files'" :ready="ready && !busy" />
    <p v-show="pane === 'terminal'" v-if="active?.error || preferences.error.value || error" class="terminal-warning" role="alert">{{ active?.error || preferences.error.value || (error && presentError(error).message) }}</p>
    <TerminalSettingsDialog :open="settingsOpen" :preferences="preferences" @close="closeSettings" />
    <BaseDialog :open="disconnectOpen" title="断开 SSH 连接？" :busy="busy" @close="disconnectOpen = false">
      <p>与 {{ server.name }} 的连接和所有远程终端将结束。</p>
      <label v-if="requireStopConfirmation"><input v-model="stopTransfers" type="checkbox" :disabled="busy" />停止该连接的传输任务后断开</label>
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton :disabled="busy" @click="disconnectOpen = false">保留连接</BaseButton><BaseButton variant="danger" :disabled="busy || (requireStopConfirmation && !stopTransfers)" @click="emit('disconnect', stopTransfers)">确认断开</BaseButton></template>
    </BaseDialog>
  </section>
</template>
