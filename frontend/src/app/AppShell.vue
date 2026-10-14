<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { connectionStateText } from "../i18n/connections";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { AppInfo } from "../ipc/commands";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { IpcClient } from "../ipc/client";
import { locale } from '../i18n/locale';
import { settingsMessages } from '../i18n/settings';
import { paletteMessages } from '../i18n/palette';
import { isTerminalTarget } from './palette';
import CredentialRevealDialog from '../dialogs/CredentialRevealDialog.vue';
import CommandPalette from '../components/base/CommandPalette.vue';
import { createMonitorApi } from "../ipc/monitor";
import { createMonitorStore } from "../stores/monitor";
import { createSftpApi } from "../ipc/sftp";
import { createTransferStore, transferFinished, type TransferChannelFactory } from "../stores/transfers";
import { createTerminalApi } from "../ipc/terminal";
import { createSettingsApi } from "../ipc/settings";
import { createCredentialRevealApi } from "../ipc/credential-reveal";
import { createBackgroundImagesApi } from "../ipc/background-images";
import { createServerAppearanceApi } from "../ipc/server-appearance";
import { createServerRuntimeStatsApi } from "../ipc/server-runtime-stats";
import { createTerminalController, type TerminalChannelFactory } from "../terminal/controller";
import { createTerminalPreferences, defaultSettings } from "../terminal/preferences";
import TerminalWorkspace from "../components/terminal/TerminalWorkspace.vue";
import { createConnectionApi } from "../ipc/connection";
import { createHostKeysApi } from "../ipc/host-keys";
import { createGeoIpApi } from '../ipc/geoip';
import { createNetworkApi } from "../ipc/network";
import { createPreflightApi } from "../ipc/preflight";
import { applyAccentColor } from "../theme/accent";
import { createConnectionStore, isFinished } from "../stores/connections";
import ConnectionDialogs from "../dialogs/ConnectionDialogs.vue";
import { createServerApi } from "../ipc/server";
import { createServerStore } from "../stores/servers";
import { createServerAppearanceStore } from "../stores/server-appearance";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseEmptyState from "../components/base/BaseEmptyState.vue";
import BaseToastViewport from "../components/base/BaseToastViewport.vue";
import { useToast } from "../composables/useToast";
import ServerDialog from "../dialogs/ServerDialog.vue";
import ConfirmDialog from "../dialogs/ConfirmDialog.vue";
import GroupDialog from "../dialogs/GroupDialog.vue";
import TopBar from "./TopBar.vue";
import Sidebar from "./Sidebar.vue";
import SettingsSidebar from "./settings/SettingsSidebar.vue";
import SettingsWorkspace from "./settings/SettingsWorkspace.vue";
import type { SettingsSection } from "./settings/settings-navigation";
import LocalBackendStatus from "./LocalBackendStatus.vue";
import DesktopWorkspaceTabs from "./DesktopWorkspaceTabs.vue";
import IdleWorkspace from "./IdleWorkspace.vue";
import ServerOverview from "../components/server-overview/ServerOverview.vue";
import { serverOverviewMessages } from "../i18n/server-overview";
import { updaterMessages } from "../i18n/updater";
import UpdateInstallConfirmDialog from "../components/update/UpdateInstallConfirmDialog.vue";
import { createNativeUpdaterAdapter, createUnsupportedUpdaterAdapter } from "../updater/updater-adapter";
import { inspectInstallActivity } from "../updater/install-safety";
import { createUpdaterController } from "../updater/updater-controller";
import type { UpdaterAdapter } from "../updater/updater-types";
import type { ConnectionTestToastResult } from "./connection-test-toast";
import type { ServerNavigationRequest } from "./server-navigation";
import { createWorkspaceUiStore } from "../stores/workspace-ui";
const t = messages(shellMessages);

const props = withDefaults(defineProps<{ client: IpcClient; readOnly?: boolean; terminalChannelFactory?: TerminalChannelFactory; transferChannelFactory?: TransferChannelFactory; backgroundImageAssetUrl?: (path: string) => string; updaterAdapter?: UpdaterAdapter }>(), { readOnly: false });
const info = ref<AppInfo | null>(null);
const updateInstallGate = ref(false);
const updaterAdapter = props.updaterAdapter ?? (props.readOnly ? createUnsupportedUpdaterAdapter() : createNativeUpdaterAdapter(props.client));
const updaterController = createUpdaterController(updaterAdapter, () => info.value?.version ?? '—');
const store = createServerStore(createServerApi(props.client));
const serverAppearances = createServerAppearanceStore(createServerAppearanceApi(props.client));
const runtimeStats = createServerRuntimeStatsApi(props.client);
const { servers, groups, pending, error, query } = store;
const connections = createConnectionStore(createConnectionApi(props.client), () => !updateInstallGate.value);
const workspaceUi = createWorkspaceUiStore();
const diagnosticServerId = ref<string | null>(null);
const sidebarVisible = ref(true);
const hostKeys = createHostKeysApi(props.client);
const network = createNetworkApi(props.client);
const geoip = createGeoIpApi(props.client);
const databaseRevision = ref(0);
const settingsInitialSection = ref<SettingsSection>('general');
function openSettings(section: SettingsSection = 'general') { settingsInitialSection.value = section; settingsOpen.value = true; }
const credentialReveal = createCredentialRevealApi(props.client);
type RevealServer = { id: string; name: string; authType: 'password' | 'privateKey'; hasSavedCredential: boolean };
const revealServer = ref<RevealServer | null>(null);
const revealDialogOpen = ref(false);
async function openCredentialReveal(serverId: string) {
  const server = servers.value.find(item => item.id === serverId);
  if (!server?.hasSavedCredential) return;
  revealServer.value = { id: server.id, name: server.name, authType: server.authType, hasSavedCredential: server.hasSavedCredential };
  await nextTick();
  revealDialogOpen.value = true;
}
async function openSecurityFromReveal() {
  revealDialogOpen.value = false;
  revealServer.value = null;
  editor.value = false;
  await nextTick();
  openSettings('security');
}
const preflight = createPreflightApi(props.client);
const terminals = createTerminalController(createTerminalApi(props.client), props.terminalChannelFactory);
const terminalPreferences = createTerminalPreferences(createSettingsApi(props.client), settings => {
  terminals.applySettings(settings);
  document.documentElement.dataset.theme = settings.theme;
  document.documentElement.dataset.density = settings.uiDensity;
  applyAccentColor(settings.accentColor, settings.customAccentColor);
  document.documentElement.lang = settings.language;
  locale.value = settings.language;
});
const backgroundImages = createBackgroundImagesApi(props.client, props.backgroundImageAssetUrl);
terminals.setCopyPreference(() => terminalPreferences.copyOnSelect.value);
const sftp = createSftpApi(props.client);
const transfers = createTransferStore(sftp, purpose => props.client.call("local_file_select", { purpose }), props.transferChannelFactory, () => !updateInstallGate.value);
const activeTransfers = computed(() => transfers.snapshots.value.filter(task => !transferFinished(task)).length);
const updateActivity = computed(() => inspectInstallActivity(connections.snapshots.value, transfers.snapshots.value, transfers.starting.value, terminals.tabs.value, connections.starting.value > 0));
const monitor = createMonitorStore(createMonitorApi(props.client));
const workspaceViews = ref<Record<string, string>>({});
const focused = ref(false);
const settingsOpen = ref(false);
const selectedId = computed(() => diagnosticServerId.value ?? workspaceUi.activeServerId.value);
const workspaces = computed(() => workspaceUi.tabs.value.flatMap(tab => {
  const snapshot = connections.snapshots.value[tab.serverId];
  if (!snapshot) return [];
  const hasTerminal = terminals.tabs.value.some(item => item.connectionId === snapshot.connectionId);
  const hasTransfer = transfers.snapshots.value.some(task => task.connectionId === snapshot.connectionId);
  return snapshot.state === 'ready' || hasTerminal || hasTransfer ? [[tab.serverId, snapshot] as const] : [];
}));
const workspaceTabItems = computed(() => workspaceUi.tabs.value.flatMap(tab => {
  const server = servers.value.find(item => item.id === tab.serverId);
  if (!server) return [];
  const snapshot = connections.snapshots.value[tab.serverId];
  return [{ key: tab.key, serverId: tab.serverId, name: server.name, host: server.host, phase: tab.phase, connectionState: snapshot && !isFinished(snapshot) ? snapshot.state : null, busy: !!connections.busy.value[tab.serverId] }];
}));
const activeSnapshot = computed(() => {
  const id = workspaceUi.activeServerId.value;
  return id ? connections.snapshots.value[id] ?? null : null;
});
const activeWorkspaceStatus = computed(() => {
  const tab = workspaceUi.activeTab.value;
  if (!tab) return '';
  if (activeSnapshot.value && !isFinished(activeSnapshot.value)) return connectionStateText(activeSnapshot.value.state);
  return tab.phase === 'failed' ? t('connectionFailed') : t('connectingWorkspace');
});
const activeWorkspaceError = computed(() => {
  const id = workspaceUi.activeServerId.value;
  if (!id) return '';
  const failure = connections.errors.value[id] ?? activeSnapshot.value?.error;
  return failure ? presentError(failure).message : '';
});
const pendingServerNavigation = ref<{ serverId: string; view: "terminal" | "monitor" | "files" } | null>(null);
const activeConnection = computed(() => {
  const id = workspaceUi.activeServerId.value;
  const snapshot = id ? connections.snapshots.value[id] : null;
  return !pending.value && !error.value && !settingsOpen.value && !diagnosticServerId.value && snapshot?.state === 'ready' ? snapshot.connectionId : null;
});
watch([connections.snapshots, connections.errors], () => {
  for (const tab of workspaceUi.tabs.value) workspaceUi.syncConnection(tab.serverId, connections.snapshots.value[tab.serverId] ?? null, !!connections.errors.value[tab.serverId]);
}, { immediate: true });
watch([activeConnection, workspaceViews, focused, settingsOpen, diagnosticServerId], () => { const id = activeConnection.value; if (!props.readOnly) monitor.activate(id, !!id && !settingsOpen.value && (workspaceViews.value[id] ?? 'terminal') !== 'files' && !focused.value); }, { immediate: true, deep: true });
let knownAppearanceIds = new Set<string>();
watch(serverAppearances.appearances, values => {
  for (const id of knownAppearanceIds) if (!values[id]) terminals.setServerAppearance(id, undefined);
  for (const [id, appearance] of Object.entries(values)) terminals.setServerAppearance(id, appearance);
  knownAppearanceIds = new Set(Object.keys(values));
}, { immediate: true });
const paletteOpen = ref(false);
const settingsWorkspaceRef = ref<InstanceType<typeof SettingsWorkspace> | null>(null);
const settingsSidebarBusy = ref(false);
const updateInstallDialogOpen = ref(false);
const updateInstallBusy = ref(false);
let pendingSettingsAction: (() => void | Promise<void>) | null = null;
function requestSettingsLeave(action: () => void | Promise<void> = () => {}) {
  if (!settingsOpen.value) { void action(); return; }
  pendingSettingsAction = action;
  settingsWorkspaceRef.value?.requestLeave();
}
function finishSettingsLeave() {
  settingsOpen.value = false;
  const action = pendingSettingsAction;
  pendingSettingsAction = null;
  if (action) void action();
}
function cancelSettingsLeave() { pendingSettingsAction = null; }
const settingsText = messages(settingsMessages); const paletteText = messages(paletteMessages);
const overviewText = messages(serverOverviewMessages);
const workspaceRefs = ref<InstanceType<typeof TerminalWorkspace>[]>([]);
const commands = computed(() => [...servers.value.map(server => ({ id: `server:${server.id}`, label: server.name, meta: `${server.username}@${server.host}`, keywords: [server.host, 'server', 'connect'] })), ...['workspace', 'monitor', 'files', 'add', 'settings', 'theme', 'language', 'clear'].map(id => ({ id, label: paletteText(id as keyof typeof paletteMessages), keywords: [id], disabled: ['workspace', 'monitor', 'files'].includes(id) ? !activeConnection.value : id === 'settings' ? props.readOnly : ['theme', 'language'].includes(id) ? props.readOnly || terminalPreferences.busy.value || !terminalPreferences.record.value : id === 'add' ? !canManage.value : id === 'clear' ? !transfers.snapshots.value.some(task => task.state === 'completed') : false }))]);
async function executeCommand(id: string) {
  paletteOpen.value = false; await nextTick();
  const connectionId = activeConnection.value;
  if (id.startsWith('server:')) requestSettingsLeave(() => { const server = servers.value.find(item => item.id === id.slice(7)); if (server) return openServerView(server, 'terminal'); });
  else if (id === 'add') requestSettingsLeave(() => openEditor());
  else if (id === 'settings') openSettings();
  else if (id === 'theme' || id === 'language') requestSettingsLeave(async () => { const current = terminalPreferences.record.value?.value; if (current && await terminalPreferences.save(id === 'theme' ? { theme: getComputedStyle(document.documentElement).colorScheme === 'dark' ? 'light' : 'dark' } : { language: current.language === 'en' ? 'zh-CN' : 'en' })) toast.success(settingsText('saved')); });
  else if (id === 'clear') requestSettingsLeave(() => { new Set(transfers.snapshots.value.map(task => task.connectionId)).forEach(id => transfers.clearCompleted(id)); toast.info(paletteText('cleared')); });
  else if (connectionId) requestSettingsLeave(async () => { const workspace = workspaceRefs.value.find(value => value.connectionId === connectionId); await workspace?.view(id === 'workspace' ? 'terminal' : id as 'monitor' | 'files'); });
}
const about = ref(false);
const editor = ref(false);
const editingId = ref<string | null>(null);
const manageGroups = ref(false);
const deleteTarget = ref<ServerProfile | null>(null);
const toast = useToast();
const updaterText = messages(updaterMessages);
const selected = computed(() => servers.value.find(server => server.id === selectedId.value));
const backendState = computed(() => pending.value ? "pending" : error.value ? "error" : "ready");
const shortcut = computed(() => info.value?.platform === "windows" ? "Ctrl K" : "⌘ K");
const canManage = computed(() => !props.readOnly && !pending.value && !error.value);

function cancelUpdateInstall() {
  if (updateInstallBusy.value) return;
  updateInstallDialogOpen.value = false;
  updaterController.cancelInstallRequest();
}

async function disconnectForUpdate() {
  const sessions = Object.entries(connections.snapshots.value).filter(([, snapshot]) => !isFinished(snapshot));
  const openTerminals = terminals.tabs.value.filter(tab => tab.state !== 'closed' && tab.state !== 'failed');
  await Promise.all(openTerminals.filter(tab => tab.state !== 'opening' && tab.state !== 'closing').map(tab => terminals.close(tab.id)));
  await Promise.all(sessions.map(async ([id, snapshot]) => {
    if (snapshot.state === 'ready') await connections.disconnect(id);
    else await connections.cancel(id);
  }));

  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const activity = updateActivity.value;
    if (activity.activeConnections.length === 0 && activity.activeTerminals.length === 0) return true;
    const ids = Object.entries(connections.snapshots.value).filter(([, snapshot]) => !isFinished(snapshot)).map(([id]) => id);
    const connectionIds = new Set([...activity.activeConnections.map(snapshot => snapshot.connectionId), ...activity.activeTerminals.map(tab => tab.connectionId)]);
    await Promise.all(ids.map(id => connections.refresh(id)));
    await Promise.all([...connectionIds].map(id => terminals.refreshConnection(id)));
    await new Promise(resolve => setTimeout(resolve, 250));
  }
  return false;
}

async function beginUpdateInstall(disconnectSessions: boolean) {
  if (updateInstallBusy.value) return;
  updateInstallBusy.value = true;
  updateInstallGate.value = true;
  let nativeInstallStarted = false;
  try {
    const settingsReady = await settingsWorkspaceRef.value?.prepareForUpdateInstall();
    if (!settingsReady) {
      updaterController.cancelInstallRequest();
      updateInstallDialogOpen.value = false;
      toast.error(updaterText('settingsSaveFailed'));
      return;
    }

    let activity = updateActivity.value;
    if (activity.connectionsStarting) {
      updaterController.cancelInstallRequest();
      updateInstallDialogOpen.value = false;
      toast.error(updaterText('connectionStarting'));
      return;
    }
    if (activity.activeTransfers.length > 0 || activity.transfersStarting) {
      updaterController.cancelInstallRequest();
      updateInstallDialogOpen.value = false;
      toast.error(updaterText('transfersActive'));
      return;
    }
    if (activity.activeConnections.length > 0 || activity.activeTerminals.length > 0) {
      if (!disconnectSessions) {
        updateInstallDialogOpen.value = true;
        return;
      }
      if (!(await disconnectForUpdate())) throw new Error('session-close-failed');
      activity = updateActivity.value;
      if (activity.activeConnections.length > 0 || activity.activeTerminals.length > 0) throw new Error('session-close-failed');
    }
    if (activity.connectionsStarting) throw new Error('connection-starting');
    if (activity.activeTransfers.length > 0 || activity.transfersStarting) throw new Error('transfers-active');

    const platform = info.value?.platform;
    if (!platform) throw new Error('unsupported-platform');
    await updaterController.installConfirmed(platform);
    nativeInstallStarted = updaterController.adapterKind === 'native' && updaterController.state.value.phase === 'installing';
    if (!nativeInstallStarted) updateInstallDialogOpen.value = false;
  } catch (error) {
    updaterController.cancelInstallRequest();
    updateInstallDialogOpen.value = false;
    toast.error(error instanceof Error && error.message === 'transfers-active' ? updaterText('transfersActive')
      : error instanceof Error && error.message === 'unsupported-platform' ? updaterText('onlyDesktop')
        : error instanceof Error && error.message === 'connection-starting' ? updaterText('connectionStarting')
          : updaterText('sessionCloseFailed'));
  } finally {
    if (!nativeInstallStarted) {
      updateInstallBusy.value = false;
      updateInstallGate.value = false;
    }
  }
}

function requestUpdateInstall() {
  if (updaterController.state.value.phase !== 'confirmingInstall' || updateInstallBusy.value) return;
  const activity = updateActivity.value;
  if (activity.connectionsStarting) {
    updaterController.cancelInstallRequest();
    toast.error(updaterText('connectionStarting'));
    return;
  }
  if (activity.activeTransfers.length > 0 || activity.transfersStarting) {
    updaterController.cancelInstallRequest();
    toast.error(updaterText('transfersActive'));
    return;
  }
  if (activity.activeConnections.length > 0 || activity.activeTerminals.length > 0) {
    updateInstallDialogOpen.value = true;
    return;
  }
  void beginUpdateInstall(false);
}

function deferUpdateInstall() { toast.info(updaterText('installDeferred')); }

async function openUpdatesFromAbout() {
  about.value = false;
  await nextTick();
  openSettings('general');
  await nextTick();
  document.querySelector<HTMLButtonElement>('#settings-updates button')?.focus();
}

function openEditor(id: string | null = null) {
  if (!canManage.value) return;
  editingId.value = id;
  editor.value = true;
}

async function onRemoved(message: string) {
  if (deleteTarget.value) {
    serverAppearances.remove(deleteTarget.value.id);
    workspaceUi.close(deleteTarget.value.id);
    if (diagnosticServerId.value === deleteTarget.value.id) diagnosticServerId.value = null;
  }
  toast.success(message);
  await nextTick();
  document.querySelector<HTMLButtonElement>(".shell-server-item[aria-current='page']")?.focus();
}

function selectServer(id: string | null) {
  if (!id) { diagnosticServerId.value = null; pendingServerNavigation.value = null; return; }
  const server = servers.value.find(item => item.id === id);
  if (server) void openServerView(server, 'terminal');
}

async function revealWorkspaceView(serverId: string, view: "terminal" | "monitor" | "files") {
  diagnosticServerId.value = null;
  workspaceUi.select(serverId);
  workspaceUi.setPane(serverId, view);
  await nextTick();
  const snapshot = connections.snapshots.value[serverId];
  if (snapshot?.state !== "ready") return;
  const workspace = workspaceRefs.value.find(value => value.connectionId === snapshot.connectionId);
  await workspace?.view(view);
}

async function openServerView(server: ServerProfile, view: "terminal" | "monitor" | "files") {
  diagnosticServerId.value = null;
  workspaceUi.open(server.id, view);
  pendingServerNavigation.value = null;
  const snapshot = connections.snapshots.value[server.id];
  if (snapshot?.state === "ready") {
    await revealWorkspaceView(server.id, view);
    return;
  }
  if (snapshot && !isFinished(snapshot)) {
    pendingServerNavigation.value = { serverId: server.id, view };
    return;
  }
  if (snapshot) connections.dismiss(server.id);
  pendingServerNavigation.value = { serverId: server.id, view };
  await connections.start(server);
  const next = connections.snapshots.value[server.id];
  if (next?.state === "ready") await revealWorkspaceView(server.id, view);
  else if (!next || isFinished(next)) pendingServerNavigation.value = null;
}

function handleServerNavigation({ action, server }: ServerNavigationRequest) {
  if (action === "edit") {
    pendingServerNavigation.value = null;
    openEditor(server.id);
  } else if (action === "remove") {
    pendingServerNavigation.value = null;
    deleteTarget.value = { ...server };
  } else if (action === "details") {
    pendingServerNavigation.value = null;
    diagnosticServerId.value = server.id;
  } else {
    const view = action === "workspace" ? "terminal" : action;
    void openServerView(server, view);
  }
}

function finishWorkspaceClose(serverId: string) {
  const snapshot = connections.snapshots.value[serverId];
  if (snapshot && !isFinished(snapshot)) return false;
  if (snapshot) connections.dismiss(serverId);
  workspaceUi.close(serverId);
  if (diagnosticServerId.value === serverId) diagnosticServerId.value = null;
  if (pendingServerNavigation.value?.serverId === serverId) pendingServerNavigation.value = null;
  return true;
}

async function closeWorkspace(serverId: string) {
  if (connections.busy.value[serverId]) return;
  const snapshot = connections.snapshots.value[serverId];
  if (!snapshot || isFinished(snapshot)) { finishWorkspaceClose(serverId); return; }
  if (snapshot.state === 'ready') {
    const workspace = workspaceRefs.value.find(value => value.serverId === serverId);
    if (workspace) { workspace.requestWorkspaceClose(); return; }
    await handleWorkspaceDisconnect(serverId, false, true);
    return;
  }
  workspaceUi.setPhase(serverId, 'closing');
  await connections.cancel(serverId);
  if (!finishWorkspaceClose(serverId)) workspaceUi.syncConnection(serverId, connections.snapshots.value[serverId] ?? null, !!connections.errors.value[serverId]);
}

async function handleWorkspaceDisconnect(serverId: string, stopActiveTransfers: boolean, closeAfterDisconnect = false) {
  await connections.disconnect(serverId, stopActiveTransfers);
  const snapshot = connections.snapshots.value[serverId];
  if (closeAfterDisconnect && snapshot && isFinished(snapshot)) finishWorkspaceClose(serverId);
}

function toggleSidebar() { sidebarVisible.value = !sidebarVisible.value; }

function retryActiveWorkspace() {
  const tab = workspaceUi.activeTab.value;
  const server = tab && servers.value.find(item => item.id === tab.serverId);
  if (server) void openServerView(server, tab.activePane);
}

watch(() => {
  const target = pendingServerNavigation.value;
  return target ? connections.snapshots.value[target.serverId] : null;
}, async snapshot => {
  const target = pendingServerNavigation.value;
  if (!target || !snapshot) return;
  if (snapshot.state === "ready") {
    pendingServerNavigation.value = null;
    await revealWorkspaceView(target.serverId, target.view);
  } else if (isFinished(snapshot)) pendingServerNavigation.value = null;
});

function onConnectionTestResult(result: ConnectionTestToastResult) {
  if (result.kind === "success") toast.success(result.description, { title: result.title });
  else toast.error(result.description, { title: result.title });
}

async function copyOverviewValue(kind: "address" | "ssh", value: string) {
  try {
    await navigator.clipboard.writeText(value);
    toast.success(overviewText(kind === "address" ? "addressCopied" : "sshCommandCopied"));
  } catch {
    toast.error(overviewText("copyFailed"));
  }
}

async function load() {
  pending.value = true;
  error.value = null;
  try {
    info.value = await props.client.getInfo();
    updaterController.setCurrentVersion(info.value.version);
    await store.load();
    await serverAppearances.load();
  } catch (failure) {
    error.value = presentError(mapError(failure));
  } finally {
    pending.value = false;
  }
}

watch(() => selectedId.value ? connections.snapshots.value[selectedId.value]?.state : undefined, async state => {
  if (!state || !['ready', 'closed', 'failed', 'cancelled'].includes(state)) return;
  await nextTick();
  // Start/Cancel controls disappear as Core completes an attempt. Restore focus to the current action.
  if (document.activeElement === document.body && !document.querySelector('[role="dialog"]')) {
    document.querySelector<HTMLButtonElement>('.connection-actions button:not(:disabled)')?.focus();
  }
});

function onKeydown(event: KeyboardEvent) {
  if (isTerminalTarget(event.target) || document.querySelector('[role="dialog"]') || connections.challenge.value || about.value || editor.value || manageGroups.value || deleteTarget.value) return;
  if (event.target instanceof HTMLElement && (event.target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName))) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    paletteOpen.value = true;
  } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
    event.preventDefault();
    toggleSidebar();
  }
}
onMounted(() => { document.addEventListener("keydown", onKeydown); void load(); if (!props.readOnly) void terminalPreferences.load(); });
onBeforeUnmount(() => { document.removeEventListener("keydown", onKeydown); void updaterController.dispose(); connections.dispose(); terminals.dispose(); transfers.dispose(); monitor.dispose(); });
</script>

<template>
  <div class="application-shell" :class="{ 'terminal-focused': focused, 'is-explorer-hidden': !sidebarVisible }" :data-sidebar-width="terminalPreferences.record.value?.value.sidebarWidth ?? 'standard'">
    <TopBar v-model:query="query" :shortcut="shortcut" :settings-enabled="!readOnly" :explorer-visible="sidebarVisible" @about="requestSettingsLeave(() => { about = true; })" @settings="openSettings()" @palette="paletteOpen = true" @toggle-explorer="toggleSidebar" />
    <div class="shell-content" :class="{ 'is-explorer-hidden': !sidebarVisible }">
      <SettingsSidebar v-if="settingsOpen" :section="settingsInitialSection" :has-geo-ip="!!geoip" :busy="settingsSidebarBusy" @update:section="settingsInitialSection = $event" @back="requestSettingsLeave()" />
      <Sidebar v-else v-show="sidebarVisible" v-model:query="query" :servers="servers" :groups="groups" :snapshots="connections.snapshots.value" :selected-id="selectedId" :pending="pending" :failed="!!error" :can-manage="canManage" @select="selectServer($event)" @add="openEditor()" @groups="manageGroups = true" @server-action="handleServerNavigation" />
      <main class="shell-main" :class="{ 'has-terminal-workspace': !!workspaceUi.activeTab.value && !settingsOpen && !diagnosticServerId }" :aria-busy="pending">
        <DesktopWorkspaceTabs v-if="!settingsOpen" :tabs="workspaceTabItems" :active-key="workspaceUi.activeServerId.value" @select="workspaceUi.select($event); diagnosticServerId = null" @close="closeWorkspace" />
        <div class="shell-main-content" :class="{ 'has-terminal-workspace': !!workspaceUi.activeTab.value && !settingsOpen && !diagnosticServerId }">
        <SettingsWorkspace v-if="settingsOpen" ref="settingsWorkspaceRef" :active="settingsOpen" :section="settingsInitialSection" :preferences="terminalPreferences" :background-images="backgroundImages" :geoip="geoip" :credential-reveal="credentialReveal" :updater="updaterController" :current-version="info?.version ?? '—'" :update-install-busy="updateInstallBusy" @update:section="settingsInitialSection = $event" @leave="finishSettingsLeave" @leave-cancelled="cancelSettingsLeave" @database-changed="databaseRevision++" @saved="toast.success(settingsText('saved'))" @busy-change="settingsSidebarBusy = $event" @update-install-request="requestUpdateInstall" @update-install-deferred="deferUpdateInstall" />
        <template v-else>
          <BaseEmptyState v-if="pending" :title="t('loadingLocalData')" />
          <BaseEmptyState v-else-if="error" :title="t('localServiceUnavailable')" :description="error.message">
            <BaseButton @click="load">{{ t('retry') }}</BaseButton>
          </BaseEmptyState>
          <ServerOverview v-else-if="diagnosticServerId && selected" :server="selected" :store="connections" :host-key-api="hostKeys" :network-api="network" :preflight-api="preflight" :runtime-stats-api="runtimeStats" :read-only="!canManage" :geo-ip-lookup-mode="terminalPreferences.record.value?.value.geoIpLookupMode ?? defaultSettings.geoIpLookupMode" @back="selectServer(null)" @edit="openEditor($event)" @remove="deleteTarget = { ...$event }" :database-revision="databaseRevision" @open-settings="openSettings('network')" @copy="copyOverviewValue" @test-result="onConnectionTestResult" />
          <BaseEmptyState v-else-if="workspaceUi.activeTab.value && activeSnapshot?.state !== 'ready'" :title="activeWorkspaceStatus" :description="activeWorkspaceError || undefined">
            <BaseButton v-if="workspaceUi.activeTab.value.phase === 'failed'" @click="retryActiveWorkspace">{{ t('retry') }}</BaseButton>
            <BaseButton variant="secondary" @click="closeWorkspace(workspaceUi.activeTab.value.serverId)">{{ t('closeWorkspace', { name: servers.find(item => item.id === workspaceUi.activeTab.value?.serverId)?.name ?? '' }) }}</BaseButton>
          </BaseEmptyState>
          <IdleWorkspace v-else-if="!workspaceUi.activeTab.value" :has-servers="servers.length > 0" :shortcut="shortcut" :sidebar-shortcut="info?.platform === 'windows' ? 'Ctrl B' : '⌘ B'" @add="requestSettingsLeave(() => openEditor())" />
        </template>
        <TerminalWorkspace v-for="[id, snapshot] in workspaces" ref="workspaceRefs" :key="snapshot.connectionId" v-show="!pending && !error && !settingsOpen && !diagnosticServerId && workspaceUi.activeServerId.value === id" :server="servers.find(server => server.id === id)!" :server-appearance="serverAppearances.appearances.value[id]" :snapshot="snapshot" :controller="terminals" :sftp="sftp" :transfers="transfers" :monitor="monitor" :preferences="terminalPreferences" :background-images="backgroundImages" :visible="!pending && !error && !settingsOpen && !diagnosticServerId && workspaceUi.activeServerId.value === id" :busy="!!connections.busy.value[id]" :error="connections.errors.value[id]" @settings="openSettings('terminal')" @disconnect="(stopTransfers, closeAfterDisconnect) => handleWorkspaceDisconnect(id, stopTransfers, closeAfterDisconnect)" @focus-mode="focused = $event" @view="workspaceViews[snapshot.connectionId] = $event; workspaceUi.setPane(id, $event)" />
        </div>
      </main>
      <LocalBackendStatus :state="backendState" :version="info?.version" :workspace-count="workspaceUi.tabs.value.length" :transfer-count="activeTransfers" />
    </div>
    <BaseDialog :open="about" size="compact" :title="t('aboutMauLink')" @close="about = false">
      <p>{{ t('aboutLead') }}</p>
      <dl class="shell-about-details"><dt>{{ t('version') }}</dt><dd>{{ info?.version ?? '—' }}</dd><dt>{{ t('platform') }}</dt><dd>{{ info?.platform ?? '—' }}</dd><dt>{{ t('architecture') }}</dt><dd>{{ info?.architecture ?? '—' }}</dd></dl>
      <template #footer><BaseButton variant="secondary" @click="openUpdatesFromAbout">{{ updaterText('check') }}</BaseButton></template>
    </BaseDialog>
    <UpdateInstallConfirmDialog :open="updateInstallDialogOpen" :busy="updateInstallBusy" :active-connections="updateActivity.activeConnections.length" :active-terminals="updateActivity.activeTerminals.length" @cancel="cancelUpdateInstall" @confirm="beginUpdateInstall(true)" />
    <ServerDialog :language="locale" :open="editor" :server-id="editingId" :store="store" :appearance-store="serverAppearances" :background-images="backgroundImages" :connection-store="connections" :global-settings="terminalPreferences.record.value?.value ?? defaultSettings" @close="editor = false" @reveal-credential="openCredentialReveal" @saved="toast.success($event)" @test-result="onConnectionTestResult" />
    <CredentialRevealDialog :open="revealDialogOpen" :server="revealServer" :api="credentialReveal" @close="revealDialogOpen = false" @open-security="openSecurityFromReveal" />
    <ConfirmDialog :language="locale" :server="deleteTarget" :store="store" @close="deleteTarget = null" @removed="onRemoved" />
    <GroupDialog :language="locale" :open="manageGroups" :store="store" @close="manageGroups = false" @saved="toast.success($event)" />
    <ConnectionDialogs :store="connections" :servers="servers" :suspended="settingsOpen || revealDialogOpen || paletteOpen || about || (editor && !connections.draftTestActive.value) || manageGroups || !!deleteTarget" />
    <CommandPalette :open="paletteOpen" :commands="commands" @close="paletteOpen = false" @execute="executeCommand" />
    <BaseToastViewport :queue="toast" :position="terminalPreferences.record.value?.value.toastPosition ?? 'topRight'" />
  </div>
</template>
