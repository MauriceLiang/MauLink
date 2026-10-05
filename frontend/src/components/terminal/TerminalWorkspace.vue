<script setup lang="ts">
import BaseCheckbox from "../base/BaseCheckbox.vue";
import { messages } from "../../i18n/locale";
import { terminalMessages } from "../../i18n/terminal";
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
import type { BackgroundImagesApi } from "../../ipc/background-images";
import { connectionStateText } from "../../i18n/connections";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import BaseIconButton from "../base/BaseIconButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import TerminalTabs from "./TerminalTabs.vue";
import XtermHost from "./XtermHost.vue";
import TerminalSettingsDialog from "../../dialogs/TerminalSettingsDialog.vue";
import { mapError } from "../../errors/mapper";
import { terminalBackgroundImageStyle, terminalBackgroundOverlayStyle } from "../../terminal/background";
const t = messages(terminalMessages);
const props = defineProps<{ server: ServerProfile; snapshot: ConnectionSnapshot; controller: TerminalController; sftp: ReturnType<typeof createSftpApi>; transfers: TransferStore; monitor: MonitorStore; preferences: TerminalPreferences; backgroundImages: BackgroundImagesApi; visible: boolean; busy: boolean; error?: AppError | null }>();
const emit = defineEmits<{ home: []; disconnect: [stopActiveTransfers: boolean]; focusMode: [enabled: boolean]; view: [pane: 'terminal' | 'files' | 'monitor'] }>();
const root = ref<HTMLElement | null>(null);
const activeId = ref<string | null>(null);
const settingsOpen = ref(false); const disconnectOpen = ref(false); const focused = ref(false);
const tabs = computed(() => props.controller.tabs.value.filter(tab => tab.connectionId === props.snapshot.connectionId));
const active = computed(() => tabs.value.find(tab => tab.id === activeId.value));
const ready = computed(() => props.snapshot.state === 'ready');
const pane = ref<'terminal' | 'files' | 'monitor'>('terminal'); const filesVisited = ref(false); const filesExpanded = ref(false); const stopTransfers = ref(false);
const activeTransfers = computed(() => props.transfers.snapshots.value.filter(task => task.connectionId === props.snapshot.connectionId && !transferFinished(task)).length);
const filePageInfo = ref('');
const backgroundImageUrl = ref(''); const backgroundError = ref('');
const currentBackground = computed(() => props.preferences.record.value?.value.terminalBackgroundImage);
const terminalBackgroundStyle = computed(() => {
  const settings = currentBackground.value;
  if (!backgroundImageUrl.value || !settings) return {};
  return terminalBackgroundImageStyle(backgroundImageUrl.value, settings);
});
const terminalOverlayStyle = computed(() => currentBackground.value ? terminalBackgroundOverlayStyle(currentBackground.value) : {});
const requireStopConfirmation = computed(() => activeTransfers.value > 0 || props.error?.messageKey === 'errors.activeTransfersRequireConfirmation');
watch(() => {
  const value = props.preferences.record.value?.value;
  return { mode: value?.terminalThemeMode, imageId: value?.terminalBackgroundImage.imageId };
}, async target => {
  backgroundImageUrl.value = ''; backgroundError.value = '';
  if (target.mode !== 'image' || !target.imageId) return;
  const requestedId = target.imageId;
  try {
    const image = await props.backgroundImages.resolve(requestedId);
    const current = props.preferences.record.value?.value;
    if (current?.terminalThemeMode === 'image' && current.terminalBackgroundImage.imageId === requestedId) backgroundImageUrl.value = image.src;
  } catch (reason) {
    const current = props.preferences.record.value?.value;
    if (current?.terminalThemeMode === 'image' && current.terminalBackgroundImage.imageId === requestedId) backgroundError.value = presentError(mapError(reason)).message;
  }
}, { immediate: true });
async function view(value: 'terminal' | 'files' | 'monitor') { pane.value = value; emit('view', value); if (value !== 'terminal') { if (value === 'files') filesVisited.value = true; focused.value = false; emit('focusMode', false); } else { await nextTick(); if (activeId.value) props.controller.focus(activeId.value); } }
function toggleFiles() { filesExpanded.value = !filesExpanded.value; if (filesExpanded.value) filesVisited.value = true; }
let initialized = false; let initializing = false;
async function create() {
  if (!ready.value || props.busy) return;
  activeId.value = props.controller.create(props.snapshot.connectionId);
  await nextTick(); props.controller.focus(activeId.value);
}
watch([() => props.visible, () => props.busy, ready], async ([visible, busy]) => {
  if (!visible) { focused.value = false; emit('focusMode', false); return; }
  if (!ready.value || busy || initializing) return;
  if (!initialized) { initializing = true; try { if (!props.preferences.record.value) await props.preferences.load(); if (props.visible && ready.value && !props.busy) { initialized = true; await create(); } } finally { initializing = false; } }
  else { await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
}, { immediate: true });
watch(ready, value => { if (!value) { void props.controller.refreshConnection(props.snapshot.connectionId); focused.value = false; emit('focusMode', false); } });
async function activate(id: string, focus = true) { activeId.value = id; await nextTick(); if (focus) props.controller.focus(id); }
async function close(id: string) { if (await props.controller.close(id)) { if (activeId.value === id) activeId.value = tabs.value.at(-1)?.id ?? null; await nextTick(); if (activeId.value) props.controller.focus(activeId.value); else root.value?.querySelector<HTMLButtonElement>('.terminal-new')?.focus(); } }
async function toggleFocus() { focused.value = !focused.value; emit('focusMode', focused.value); await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
async function closeSettings() { settingsOpen.value = false; await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
function disconnect() { stopTransfers.value = false; if (requireStopConfirmation.value || (props.preferences.record.value?.value.confirmBeforeDisconnect ?? true)) disconnectOpen.value = true; else emit('disconnect', false); }
watch(() => props.busy, busy => { if (!busy && !ready.value) disconnectOpen.value = false; });
defineExpose({ connectionId: props.snapshot.connectionId, view });
</script>
<template>
  <section ref="root" class="terminal-workspace" :class="{ 'is-focused': focused, 'is-files': pane === 'files', 'is-monitor': pane === 'monitor' }" :aria-label="t('workspace', {name: server.name})">
    <header class="workspace-heading"><BaseIconButton class="workspace-heading-action" :label="t('backToServers')" @click="emit('home')"><BaseIcon name="arrow-left" /></BaseIconButton><div><h1>{{ server.name }}</h1><span>{{ server.username }}@{{ server.host }}:{{ server.port }}</span></div><span class="workspace-connection-state">{{ connectionStateText(snapshot.state) }}</span><BaseIconButton class="workspace-heading-action" :label="t('disconnect')" :disabled="!ready || busy || transfers.starting.value[snapshot.connectionId]" @click="disconnect"><BaseIcon name="power" /></BaseIconButton><div class="workspace-view-tabs" role="group" :aria-label="t('workspace', {name: server.name})"><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'terminal'" @click="view('terminal')">{{ t('terminal') }}</BaseButton><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'files'" @click="view('files')">{{ t('files') }}</BaseButton><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'monitor'" @click="view('monitor')">{{ t('monitor') }}</BaseButton></div></header>
    <div v-show="pane === 'terminal'" class="terminal-toolbar"><TerminalTabs :tabs="tabs" :active-id="activeId" @activate="activate" @close="close" /><BaseIconButton class="terminal-new terminal-toolbar-action" :label="t('newTerminal')" :disabled="!ready || busy" @click="create"><BaseIcon name="plus" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="focused ? t('exitFocus') : t('focus')" :disabled="!active" :aria-pressed="focused" @click="toggleFocus"><BaseIcon :name="focused ? 'minimize' : 'maximize'" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="t('terminalSettings')" @click="settingsOpen = true"><BaseIcon name="settings" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="t('clearTerminal')" :disabled="!active" @click="activeId && controller.clear(activeId)"><BaseIcon name="eraser" /></BaseIconButton></div>
    <div v-show="pane === 'terminal' || pane === 'files'" class="terminal-main" :class="{ 'has-inline-files': pane === 'terminal' && filesExpanded, 'is-full-files': pane === 'files' }">
      <div v-show="pane === 'terminal'" class="terminal-stack"><div v-if="backgroundImageUrl" class="terminal-background-layer" aria-hidden="true" :style="terminalBackgroundStyle"></div><div v-if="backgroundImageUrl" class="terminal-background-overlay" aria-hidden="true" :style="terminalOverlayStyle"></div><XtermHost v-for="tab in tabs" :key="tab.id" v-show="tab.id === activeId" :id="tab.id" :controller="controller" :active="visible && pane === 'terminal' && tab.id === activeId" /><p v-if="!tabs.length" class="terminal-empty">{{ ready ? t('selectNewTerminalToOpenAShell') : t('sshConnectionEnded') }}</p></div>
      <MonitorView v-show="pane === 'terminal'" :store="monitor" :connection-id="snapshot.connectionId" :ready="ready" quick @full="view('monitor')" />
      <div v-if="filesVisited" v-show="pane === 'files' || (pane === 'terminal' && filesExpanded)" class="terminal-inline-files">
        <FilesPanel :api="sftp" :transfers="transfers" :preferences="preferences" :connection-id="snapshot.connectionId" :visible="visible && (pane === 'files' || (pane === 'terminal' && filesExpanded))" :ready="ready && !busy" :compact-footer="pane === 'terminal' && filesExpanded" @pagination="filePageInfo = $event" />
      </div>
    </div>
    <footer v-show="pane === 'terminal'" class="terminal-footer" role="status"><span v-if="filesExpanded && filePageInfo">{{ filePageInfo }}</span><span>{{ active ? active.state === 'running' ? active.inputPaused ? t('inputPaused') : t('shellReady') : t('sessionStopped') : t('waitingForShell') }}</span><span>{{ active?.columns ?? '—' }} × {{ active?.rows ?? '—' }}</span><span>{{ t(tabs.length === 1 ? 'oneTerminal' : 'terminalCount', {count: tabs.length}) }}</span><BaseIconButton class="terminal-files-toggle" :label="t(filesExpanded ? 'collapseFiles' : 'expandFiles')" :disabled="!ready || busy" :aria-pressed="filesExpanded" @click="toggleFiles"><BaseIcon name="folder" /></BaseIconButton></footer>
    <MonitorView v-show="pane === 'monitor'" :store="monitor" :connection-id="snapshot.connectionId" :ready="ready" />
    <p v-show="pane === 'terminal'" v-if="active?.error || preferences.error.value || backgroundError || error" class="terminal-warning" role="alert"><span v-if="active?.error">{{ active.error }}</span><span v-if="preferences.error.value">{{ preferences.error.value }}</span><span v-if="backgroundError">{{ backgroundError }}</span><span v-if="error">{{ presentError(error).message }}</span></p>
    <p v-show="pane === 'terminal'" v-if="controller.requiresReopen.value" class="terminal-background-notice" role="status">{{ t('imageReopenWarning') }}</p>
    <TerminalSettingsDialog :open="settingsOpen" :preferences="preferences" :background-images="backgroundImages" @close="closeSettings" />
    <BaseDialog :open="disconnectOpen" :title="t('disconnectSSH')" :busy="busy" @close="disconnectOpen = false">
      <p>{{ t('disconnectNote', {name: server.name}) }}</p>
      <BaseCheckbox v-if="requireStopConfirmation" v-model="stopTransfers" :disabled="busy" :label="t('stopTransfersForThisConnectionBeforeDisconnecting')" />
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton :disabled="busy" @click="disconnectOpen = false">{{ t('keepConnection') }}</BaseButton><BaseButton variant="danger" :disabled="busy || (requireStopConfirmation && !stopTransfers)" @click="emit('disconnect', stopTransfers)">{{ t('confirmDisconnect') }}</BaseButton></template>
    </BaseDialog>
  </section>
</template>
