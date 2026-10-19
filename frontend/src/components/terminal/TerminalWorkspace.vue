<script setup lang="ts">
import BaseCheckbox from "../base/BaseCheckbox.vue";
import { messages } from "../../i18n/locale";
import { terminalMessages } from "../../i18n/terminal";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import MonitorView from "../monitor/MonitorView.vue";
import Inspector from "../monitor/Inspector.vue";
import type { MonitorStore } from "../../stores/monitor";
import FilesPanel from "../files/FilesPanel.vue";
import type { createSftpApi } from "../../ipc/sftp";
import { transferFinished, type TransferStore } from "../../stores/transfers";
import { presentError } from "../../errors/presenter";
import type { AppError } from "../../../../contracts/v1/AppError";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { ServerAppearance } from "../../../../contracts/v1/ServerAppearance";
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
import { mapError } from "../../errors/mapper";
import { terminalBackgroundImageStyle, terminalBackgroundOverlayStyle } from "../../terminal/background";
import { resolveTerminalAppearance } from "../../terminal/appearance";
import { resolveTerminalTheme } from "../../terminal/theme";
import { DEFAULT_INLINE_FILES_PREFERENCE, normalizeInlineFilesRatio, readInlineFilesPreference, resolveInlineFilesLayout, saveInlineFilesPreference } from "../../stores/workspace-layout";
const t = messages(terminalMessages);
const props = defineProps<{ server: ServerProfile; serverAppearance?: ServerAppearance; snapshot: ConnectionSnapshot; controller: TerminalController; sftp: ReturnType<typeof createSftpApi>; transfers: TransferStore; monitor: MonitorStore; preferences: TerminalPreferences; backgroundImages: BackgroundImagesApi; visible: boolean; busy: boolean; error?: AppError | null }>();
type DisconnectDisposition = 'stay' | 'close-workspace' | 'return-home';
const emit = defineEmits<{ settings: []; disconnect: [stopActiveTransfers: boolean, disposition: DisconnectDisposition]; focusMode: [enabled: boolean]; view: [pane: 'terminal' | 'files' | 'monitor'] }>();
const root = ref<HTMLElement | null>(null);
const terminalMain = ref<HTMLElement | null>(null);
const activeId = ref<string | null>(null);
const disconnectOpen = ref(false); const focused = ref(false);
const inspectorVisible = ref(false);
const disconnectDisposition = ref<DisconnectDisposition>('stay');
const tabs = computed(() => props.controller.tabs.value.filter(tab => tab.connectionId === props.snapshot.connectionId));
const active = computed(() => tabs.value.find(tab => tab.id === activeId.value));
const ready = computed(() => props.snapshot.state === 'ready');
const pane = ref<'terminal' | 'files' | 'monitor'>('terminal'); const filesVisited = ref(false); const filesExpanded = ref(false); const filesRatio = ref(DEFAULT_INLINE_FILES_PREFERENCE.ratio); const mainHeight = ref(0); const draftFilesRatio = ref<number | null>(null); const stopTransfers = ref(false);
const compactDensity = ref(typeof document !== 'undefined' && document.documentElement.dataset.density === 'compact');
const filesLayout = computed(() => resolveInlineFilesLayout(mainHeight.value, draftFilesRatio.value ?? filesRatio.value, compactDensity.value));
const inlineFilesExpanded = computed(() => pane.value === 'terminal' && filesExpanded.value && filesLayout.value.available && !focused.value);
const filesVisible = computed(() => pane.value === 'files' || inlineFilesExpanded.value);
const terminalMainStyle = computed(() => inlineFilesExpanded.value ? { '--inline-files-height': `${filesLayout.value.filesHeight}px` } : undefined);
const directorySyncing = ref(false); const directoryWarning = ref('');
const requestedDirectory = ref<{ path: string; revision: number } | null>(null);
let directoryGeneration = 0; let directoryRevision = 0; let trackedTerminal: string | null = null;
let lastDirectoryContext: { id: string; path: string } | null = null;
const activeTransfers = computed(() => props.transfers.snapshots.value.filter(task => task.connectionId === props.snapshot.connectionId && !transferFinished(task)).length);
const backgroundImageUrl = ref(''); const backgroundError = ref('');
const effectiveSettings = computed(() => {
  const global = props.preferences.record.value?.value;
  return global ? resolveTerminalAppearance(props.server.id, global, props.serverAppearance) : null;
});
const currentBackground = computed(() => effectiveSettings.value?.terminalBackgroundImage);
const terminalStackStyle = computed(() => {
  const effective = effectiveSettings.value;
  if (!effective || effective.terminalThemeMode === 'followApp') return {};
  const systemIsDark = typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches;
  return { '--color-terminal-bg': resolveTerminalTheme(effective, effective.theme, systemIsDark).theme.background };
});
const terminalBackgroundStyle = computed(() => {
  const settings = currentBackground.value;
  if (!backgroundImageUrl.value || !settings) return {};
  return terminalBackgroundImageStyle(backgroundImageUrl.value, settings);
});
const environmentLabel = computed(() => {
  const key = {
    production: "environmentProduction",
    staging: "environmentStaging",
    development: "environmentDevelopment",
    custom: "environmentCustom",
  } as const;
  const environment = props.serverAppearance?.environment;
  return environment ? t(key[environment]) : "";
});
const terminalOverlayStyle = computed(() => currentBackground.value ? terminalBackgroundOverlayStyle(currentBackground.value) : {});
const requireStopConfirmation = computed(() => activeTransfers.value > 0 || props.error?.messageKey === 'errors.activeTransfersRequireConfirmation');
watch(() => props.server.id, id => {
  const preference = readInlineFilesPreference(id);
  filesExpanded.value = preference.expanded;
  filesRatio.value = preference.ratio;
}, { immediate: true });
let mainResizeObserver: ResizeObserver | undefined;
let densityObserver: MutationObserver | undefined;
onMounted(() => {
  const updateMainHeight = () => { mainHeight.value = terminalMain.value?.getBoundingClientRect().height ?? 0; };
  if (terminalMain.value && typeof ResizeObserver !== 'undefined') {
    mainResizeObserver = new ResizeObserver(updateMainHeight);
    mainResizeObserver.observe(terminalMain.value);
  }
  if (typeof MutationObserver !== 'undefined' && typeof document !== 'undefined') {
    densityObserver = new MutationObserver(() => { compactDensity.value = document.documentElement.dataset.density === 'compact'; });
    densityObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-density'] });
  }
  updateMainHeight();
});
watch(() => {
  return { mode: effectiveSettings.value?.terminalThemeMode, imageId: effectiveSettings.value?.terminalBackgroundImage.imageId };
}, async target => {
  backgroundImageUrl.value = ''; backgroundError.value = '';
  if (target.mode !== 'image' || !target.imageId) return;
  const requestedId = target.imageId;
  try {
    const image = await props.backgroundImages.resolve(requestedId);
    const current = effectiveSettings.value;
    if (current?.terminalThemeMode === 'image' && current.terminalBackgroundImage.imageId === requestedId) backgroundImageUrl.value = image.src;
  } catch (reason) {
    const current = effectiveSettings.value;
    if (current?.terminalThemeMode === 'image' && current.terminalBackgroundImage.imageId === requestedId) backgroundError.value = presentError(mapError(reason)).message;
  }
}, { immediate: true });
async function syncFilesDirectory() {
  const token = ++directoryGeneration;
  const id = activeId.value; const connectionId = props.snapshot.connectionId;
  directorySyncing.value = true;
  let path: string | null = null;
  try { if (id) path = await props.controller.refreshWorkingDirectory(id); }
  catch { directoryWarning.value = t('workingDirectoryUnavailable'); }
  if (token !== directoryGeneration || id !== activeId.value || connectionId !== props.snapshot.connectionId || !props.visible || !filesVisible.value || !ready.value || props.busy) return;
  if (path && id) {
    lastDirectoryContext = { id, path };
    requestedDirectory.value = { path, revision: ++directoryRevision };
    directoryWarning.value = '';
  } else {
    lastDirectoryContext = null;
    requestedDirectory.value = null;
    directoryWarning.value = t('workingDirectoryUnavailable');
  }
  filesVisited.value = true;
  directorySyncing.value = false;
}
function stopDirectorySync() { ++directoryGeneration; directorySyncing.value = false; }
async function view(value: 'terminal' | 'files' | 'monitor') {
  stopDirectorySync(); pane.value = value; emit('view', value);
  if (value !== 'terminal') { focused.value = false; emit('focusMode', false); }
  if (filesVisible.value && props.visible && ready.value && !props.busy) await syncFilesDirectory();
  if (value === 'terminal') { await nextTick(); if (activeId.value) props.controller.focus(activeId.value); }
}
function persistFilesLayout() {
  saveInlineFilesPreference(props.server.id, { expanded: filesExpanded.value, ratio: filesRatio.value });
}
async function toggleFiles() {
  if (!filesLayout.value.available) { await view('files'); return; }
  stopDirectorySync(); filesExpanded.value = !filesExpanded.value; persistFilesLayout();
  if (filesExpanded.value) await syncFilesDirectory();
}
let splitterDrag: { pointerId: number; startY: number; startHeight: number; startRatio: number } | null = null;
function startFilesResize(event: PointerEvent) {
  if (!filesLayout.value.available || !filesExpanded.value || event.button !== 0) return;
  event.preventDefault();
  const handle = event.currentTarget as HTMLElement;
  splitterDrag = { pointerId: event.pointerId, startY: event.clientY, startHeight: filesLayout.value.filesHeight, startRatio: filesRatio.value };
  draftFilesRatio.value = filesRatio.value;
  handle.setPointerCapture?.(event.pointerId);
}
function moveFilesResize(event: PointerEvent) {
  if (!splitterDrag || splitterDrag.pointerId !== event.pointerId || mainHeight.value <= 0) return;
  const targetHeight = Math.min(filesLayout.value.maxFilesHeight, Math.max(filesLayout.value.minFilesHeight, splitterDrag.startHeight - (event.clientY - splitterDrag.startY)));
  draftFilesRatio.value = normalizeInlineFilesRatio(targetHeight / mainHeight.value);
}
function finishFilesResize(event: PointerEvent, commit: boolean) {
  if (!splitterDrag || splitterDrag.pointerId !== event.pointerId) return;
  const handle = event.currentTarget as HTMLElement;
  if (handle.hasPointerCapture?.(event.pointerId)) handle.releasePointerCapture(event.pointerId);
  if (commit && draftFilesRatio.value !== null) { filesRatio.value = draftFilesRatio.value; persistFilesLayout(); }
  else filesRatio.value = splitterDrag.startRatio;
  draftFilesRatio.value = null;
  splitterDrag = null;
}
function resizeFilesByKeyboard(event: KeyboardEvent) {
  const current = filesLayout.value;
  if (!current.available) return;
  let height = current.filesHeight;
  if (event.key === 'ArrowUp') height += 12;
  else if (event.key === 'ArrowDown') height -= 12;
  else if (event.key === 'Home') height = current.minFilesHeight;
  else if (event.key === 'End') height = current.maxFilesHeight;
  else return;
  event.preventDefault();
  filesRatio.value = normalizeInlineFilesRatio(height / mainHeight.value);
  persistFilesLayout();
}
function resetFilesSize() {
  if (!filesLayout.value.available) return;
  filesRatio.value = DEFAULT_INLINE_FILES_PREFERENCE.ratio;
  persistFilesLayout();
}
watch([filesVisible, activeId, () => props.visible, ready, () => props.busy], ([visible, id, workspaceVisible, connected, busy]) => {
  const next = visible && workspaceVisible && connected && !busy ? id : null;
  if (trackedTerminal === next) return;
  if (trackedTerminal) props.controller.trackWorkingDirectory(trackedTerminal, false);
  trackedTerminal = next;
  if (next) props.controller.trackWorkingDirectory(next, true);
});
watch(activeId, () => { if (props.visible && filesVisible.value && ready.value && !props.busy) void syncFilesDirectory(); else stopDirectorySync(); });
watch([() => props.visible, ready, () => props.busy], ([visible, connected, busy]) => {
  if (visible && connected && !busy && filesVisible.value) void syncFilesDirectory(); else if (!visible || !connected || busy) stopDirectorySync();
});
watch(() => active.value?.workingDirectory, path => {
  if (directorySyncing.value || !props.visible || !filesVisible.value || !ready.value || props.busy) return;
  const id = activeId.value;
  if (!path || !id) { directoryWarning.value = t('workingDirectoryUnavailable'); return; }
  directoryWarning.value = '';
  if (lastDirectoryContext?.id === id && lastDirectoryContext.path === path) return;
  lastDirectoryContext = { id, path };
  requestedDirectory.value = { path, revision: ++directoryRevision };
});
onBeforeUnmount(() => { stopDirectorySync(); mainResizeObserver?.disconnect(); densityObserver?.disconnect(); if (trackedTerminal) props.controller.trackWorkingDirectory(trackedTerminal, false); });
let initialized = false; let initializing = false;
async function create() {
  if (!ready.value || props.busy) return;
  activeId.value = props.controller.create(props.snapshot.connectionId, props.server.id);
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
async function close(id: string) { if (activeId.value === id) stopDirectorySync(); if (await props.controller.close(id)) { if (activeId.value === id) activeId.value = tabs.value.at(-1)?.id ?? null; await nextTick(); if (activeId.value) props.controller.focus(activeId.value); else root.value?.querySelector<HTMLButtonElement>('.terminal-new')?.focus(); } else if (activeId.value === id && filesVisible.value) void syncFilesDirectory(); }
async function toggleFocus() {
  focused.value = !focused.value;
  emit('focusMode', focused.value);
  await nextTick();
  await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
  if (activeId.value) props.controller.focus(activeId.value);
}
function requestDisconnect(disposition: DisconnectDisposition = 'stay') {
  stopTransfers.value = false;
  disconnectDisposition.value = disposition;
  if (requireStopConfirmation.value || (props.preferences.record.value?.value.confirmBeforeDisconnect ?? true)) disconnectOpen.value = true;
  else emit('disconnect', false, disposition);
}
function disconnect() { requestDisconnect('return-home'); }
function requestWorkspaceClose() { requestDisconnect('close-workspace'); }
function cancelDisconnect() { disconnectOpen.value = false; disconnectDisposition.value = 'stay'; }
function confirmDisconnect() { emit('disconnect', stopTransfers.value, disconnectDisposition.value); disconnectOpen.value = false; disconnectDisposition.value = 'stay'; }
watch(() => props.busy, busy => { if (!busy && !ready.value) disconnectOpen.value = false; });
defineExpose({ serverId: props.server.id, connectionId: props.snapshot.connectionId, view, requestWorkspaceClose });
</script>
<template>
  <section ref="root" class="terminal-workspace" :class="{ 'is-focused': focused, 'is-files': pane === 'files', 'is-monitor': pane === 'monitor' }" :aria-label="t('workspace', {name: server.name})">
    <header class="workspace-heading"><div><h1>{{ server.name }}</h1><span>{{ server.username }}@{{ server.host }}:{{ server.port }}</span><span v-if="environmentLabel || serverAppearance?.labelColor" class="workspace-server-environment"><i :style="serverAppearance?.labelColor ? { backgroundColor: serverAppearance.labelColor } : undefined"></i>{{ environmentLabel }}</span></div><span class="workspace-connection-state">{{ connectionStateText(snapshot.state) }}</span><div class="workspace-view-tabs" role="group" :aria-label="t('workspace', {name: server.name})"><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'terminal'" @click="view('terminal')">{{ t('terminal') }}</BaseButton><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'files'" @click="view('files')">{{ t('files') }}</BaseButton><BaseButton class="workspace-view-tab" :aria-pressed="pane === 'monitor'" @click="view('monitor')">{{ t('monitor') }}</BaseButton></div><BaseIconButton class="workspace-heading-action" :label="t(inspectorVisible ? 'hideInspector' : 'showInspector')" :aria-pressed="inspectorVisible" @click="inspectorVisible = !inspectorVisible"><BaseIcon name="monitor" /></BaseIconButton><BaseIconButton class="workspace-heading-action" :label="t('disconnect')" :disabled="!ready || busy || transfers.starting.value[snapshot.connectionId]" @click="disconnect"><BaseIcon name="power" /></BaseIconButton></header>
    <div v-show="pane === 'terminal'" class="terminal-toolbar"><TerminalTabs :tabs="tabs" :active-id="activeId" @activate="activate" @close="close" /><BaseIconButton class="terminal-new terminal-toolbar-action" :label="t('newTerminal')" :disabled="!ready || busy" @click="create"><BaseIcon name="plus" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="focused ? t('exitFocus') : t('focus')" :disabled="!active" :aria-pressed="focused" @click="toggleFocus"><BaseIcon :name="focused ? 'minimize' : 'maximize'" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="t('terminalSettings')" @click="emit('settings')"><BaseIcon name="gear" /></BaseIconButton><BaseIconButton class="terminal-toolbar-action" :label="t('clearTerminal')" :disabled="!active" @click="activeId && controller.clear(activeId)"><BaseIcon name="eraser" /></BaseIconButton></div>
    <div ref="terminalMain" v-show="pane === 'terminal' || pane === 'files'" class="terminal-main" :class="{ 'has-inline-files': inlineFilesExpanded, 'is-full-files': pane === 'files', 'has-inspector': inspectorVisible && !focused }" :style="terminalMainStyle">
      <div v-show="pane === 'terminal'" class="terminal-stack" :style="terminalStackStyle"><div v-if="backgroundImageUrl" class="terminal-background-layer" aria-hidden="true" :style="terminalBackgroundStyle"></div><div v-if="backgroundImageUrl" class="terminal-background-overlay" aria-hidden="true" :style="terminalOverlayStyle"></div><XtermHost v-for="tab in tabs" :key="tab.id" v-show="tab.id === activeId" :id="tab.id" :controller="controller" :active="visible && pane === 'terminal' && tab.id === activeId" /><p v-if="!tabs.length" class="terminal-empty">{{ ready ? t('selectNewTerminalToOpenAShell') : t('sshConnectionEnded') }}</p></div>
      <Inspector v-if="inspectorVisible && !focused && pane !== 'monitor'" :server="server" :snapshot="snapshot" :store="monitor" :ready="ready" :pane="pane" :terminals="tabs" :active-terminal="active ?? null" @close="inspectorVisible = false" @full="view('monitor')" />
      <div v-if="inlineFilesExpanded" class="terminal-files-splitter" role="separator" aria-orientation="horizontal" :aria-label="t('resizeTerminalAndFiles')" :aria-valuemin="filesLayout.minFilesHeight" :aria-valuemax="filesLayout.maxFilesHeight" :aria-valuenow="filesLayout.filesHeight" tabindex="0" @pointerdown="startFilesResize" @pointermove="moveFilesResize" @pointerup="finishFilesResize($event, true)" @pointercancel="finishFilesResize($event, false)" @keydown="resizeFilesByKeyboard" @dblclick="resetFilesSize"></div>
      <div v-if="filesVisited" v-show="filesVisible && !directorySyncing" class="terminal-inline-files">
        <FilesPanel :api="sftp" :transfers="transfers" :preferences="preferences" :connection-id="snapshot.connectionId" :visible="visible && filesVisible" :ready="ready && !busy && !directorySyncing" :requested-directory="requestedDirectory" :inline="pane === 'terminal'" @collapse="toggleFiles" />
      </div>
    </div>
    <footer v-show="pane === 'terminal'" class="terminal-footer" role="status"><span>{{ active ? active.state === 'running' ? active.inputPaused ? t('inputPaused') : t('shellReady') : t('sessionStopped') : t('waitingForShell') }}</span><span>{{ active?.columns ?? '—' }} × {{ active?.rows ?? '—' }}</span><span>{{ t(tabs.length === 1 ? 'oneTerminal' : 'terminalCount', {count: tabs.length}) }}</span><BaseIconButton v-if="!focused" class="terminal-files-toggle" :label="t(filesLayout.available ? filesExpanded ? 'collapseFiles' : 'expandFiles' : 'openFilesSeparately')" :disabled="!ready || busy" :aria-pressed="inlineFilesExpanded" @click="toggleFiles"><BaseIcon name="folder" /></BaseIconButton></footer>
    <div v-show="pane === 'monitor'" class="monitor-workspace" :class="{ 'has-inspector': inspectorVisible && !focused }">
      <MonitorView :store="monitor" :connection-id="snapshot.connectionId" :ready="ready" :server-name="server.name" />
      <Inspector v-if="inspectorVisible && !focused" :server="server" :snapshot="snapshot" :store="monitor" :ready="ready" :pane="pane" :terminals="tabs" :active-terminal="active ?? null" @close="inspectorVisible = false" @full="view('monitor')" />
    </div>
    <p v-if="directoryWarning && filesVisible" class="terminal-warning" role="status">{{ directoryWarning }}</p>
    <p v-show="pane === 'terminal'" v-if="active?.error || preferences.error.value || backgroundError || error" class="terminal-warning" role="alert"><span v-if="active?.error">{{ active.error }}</span><span v-if="preferences.error.value">{{ preferences.error.value }}</span><span v-if="backgroundError">{{ backgroundError }}</span><span v-if="error">{{ presentError(error).message }}</span></p>
    <BaseDialog :open="disconnectOpen" size="compact" variant="confirmation" :title="t('disconnectSSH')" :busy="busy" @close="cancelDisconnect">
      <p>{{ t('disconnectNote', {name: server.name}) }}</p>
      <p v-if="activeTransfers" class="disconnect-transfer-warning">{{ t('activeTransfersWarning', {count: activeTransfers}) }}</p>
      <BaseCheckbox v-if="requireStopConfirmation" v-model="stopTransfers" :disabled="busy" :label="t('stopTransfersForThisConnectionBeforeDisconnecting')" />
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton size="md" :disabled="busy" @click="cancelDisconnect">{{ t('keepConnection') }}</BaseButton><BaseButton size="md" :variant="requireStopConfirmation ? 'danger' : 'neutralPrimary'" :disabled="busy || (requireStopConfirmation && !stopTransfers)" @click="confirmDisconnect">{{ t('confirmDisconnect') }}</BaseButton></template>
    </BaseDialog>
  </section>
</template>
