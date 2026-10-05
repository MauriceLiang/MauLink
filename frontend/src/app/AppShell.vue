<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { AppInfo } from "../ipc/commands";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { IpcClient } from "../ipc/client";
import { locale } from '../i18n/locale';
import { settingsMessages } from '../i18n/settings';
import { paletteMessages } from '../i18n/palette';
import { isTerminalTarget } from './palette';
import SettingsDialog from '../dialogs/SettingsDialog.vue';
import CommandPalette from '../components/base/CommandPalette.vue';
import { createMonitorApi } from "../ipc/monitor";
import { createMonitorStore } from "../stores/monitor";
import { createSftpApi } from "../ipc/sftp";
import { createTransferStore, transferFinished, type TransferChannelFactory } from "../stores/transfers";
import { createTerminalApi } from "../ipc/terminal";
import { createSettingsApi } from "../ipc/settings";
import { createBackgroundImagesApi } from "../ipc/background-images";
import { createTerminalController, type TerminalChannelFactory } from "../terminal/controller";
import { createTerminalPreferences } from "../terminal/preferences";
import TerminalWorkspace from "../components/terminal/TerminalWorkspace.vue";
import { createConnectionApi } from "../ipc/connection";
import { createHostKeysApi } from "../ipc/host-keys";
import { createNetworkApi } from "../ipc/network";
import { createPreflightApi } from "../ipc/preflight";
import { applyAccentColor } from "../theme/accent";
import { createConnectionStore, isFinished } from "../stores/connections";
import ConnectionDialogs from "../dialogs/ConnectionDialogs.vue";
import { createServerApi } from "../ipc/server";
import { createServerStore } from "../stores/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseEmptyState from "../components/base/BaseEmptyState.vue";
import BaseToastViewport from "../components/base/BaseToastViewport.vue";
import { useToast } from "../composables/useToast";
import ServerList from "../components/server/ServerList.vue";
import ServerDialog from "../dialogs/ServerDialog.vue";
import ConfirmDialog from "../dialogs/ConfirmDialog.vue";
import GroupDialog from "../dialogs/GroupDialog.vue";
import TopBar from "./TopBar.vue";
import Sidebar from "./Sidebar.vue";
import LocalBackendStatus from "./LocalBackendStatus.vue";
import WelcomeView from "./WelcomeView.vue";
import ServerOverview from "../components/server-overview/ServerOverview.vue";
import { serverOverviewMessages } from "../i18n/server-overview";
import type { ServerNavigationRequest } from "./server-navigation";
const t = messages(shellMessages);

const props = withDefaults(defineProps<{ client: IpcClient; readOnly?: boolean; terminalChannelFactory?: TerminalChannelFactory; transferChannelFactory?: TransferChannelFactory }>(), { readOnly: false });
const info = ref<AppInfo | null>(null);
const store = createServerStore(createServerApi(props.client));
const { servers, groups, pending, error, query, filtered } = store;
const connections = createConnectionStore(createConnectionApi(props.client));
const hostKeys = createHostKeysApi(props.client);
const network = createNetworkApi(props.client);
const preflight = createPreflightApi(props.client);
const terminals = createTerminalController(createTerminalApi(props.client), props.terminalChannelFactory);
const terminalPreferences = createTerminalPreferences(createSettingsApi(props.client), settings => {
  terminals.applySettings(settings);
  document.documentElement.dataset.theme = settings.theme;
  applyAccentColor(settings.accentColor, settings.customAccentColor);
  document.documentElement.lang = settings.language;
  locale.value = settings.language;
});
const backgroundImages = createBackgroundImagesApi(props.client);
terminals.setCopyPreference(() => terminalPreferences.copyOnSelect.value);
const sftp = createSftpApi(props.client);
const transfers = createTransferStore(sftp, purpose => props.client.call("local_file_select", { purpose }), props.transferChannelFactory);
const activeTransfers = computed(() => transfers.snapshots.value.filter(task => !transferFinished(task)).length);
const monitor = createMonitorStore(createMonitorApi(props.client));
const workspaceViews = ref<Record<string, string>>({});
const focused = ref(false);
const workspaces = computed(() => Object.entries(connections.snapshots.value).filter(([id, snapshot]) => servers.value.some(server => server.id === id) && (snapshot.state === 'ready' || terminals.tabs.value.some(tab => tab.connectionId === snapshot.connectionId) || transfers.snapshots.value.some(task => task.connectionId === snapshot.connectionId))));
const selectedWorkspace = computed(() => workspaces.value.some(([id]) => id === selectedId.value));
const selectedId = ref<string | null>(null);
const pendingServerNavigation = ref<{ serverId: string; view: "terminal" | "monitor" | "files" } | null>(null);
const activeConnection = computed(() => !pending.value && !error.value && selectedId.value && connections.snapshots.value[selectedId.value]?.state === 'ready' ? connections.snapshots.value[selectedId.value]!.connectionId : null);
watch([activeConnection, workspaceViews, focused], () => { const id = activeConnection.value; if (!props.readOnly) monitor.activate(id, !!id && (workspaceViews.value[id] ?? 'terminal') !== 'files' && !focused.value); }, { immediate: true, deep: true });
const settingsOpen = ref(false); const paletteOpen = ref(false);
const settingsText = messages(settingsMessages); const paletteText = messages(paletteMessages);
const overviewText = messages(serverOverviewMessages);
const workspaceRefs = ref<InstanceType<typeof TerminalWorkspace>[]>([]);
const commands = computed(() => [...servers.value.map(server => ({ id: `server:${server.id}`, label: server.name, meta: `${server.username}@${server.host}`, keywords: [server.host, 'server', 'connect'] })), ...['workspace', 'monitor', 'files', 'add', 'settings', 'theme', 'language', 'clear'].map(id => ({ id, label: paletteText(id as keyof typeof paletteMessages), keywords: [id], disabled: ['workspace', 'monitor', 'files'].includes(id) ? !activeConnection.value : id === 'settings' ? props.readOnly : ['theme', 'language'].includes(id) ? props.readOnly || terminalPreferences.busy.value || !terminalPreferences.record.value : id === 'add' ? !canManage.value : id === 'clear' ? !transfers.snapshots.value.some(task => task.state === 'completed') : false }))]);
async function executeCommand(id: string) {
  paletteOpen.value = false; await nextTick();
  const workspace = workspaceRefs.value.find(value => value.connectionId === activeConnection.value);
  if (id.startsWith('server:')) selectServer(id.slice(7));
  else if (id === 'add') openEditor();
  else if (id === 'settings') settingsOpen.value = true;
  else if (id === 'theme' || id === 'language') { const current = terminalPreferences.record.value?.value; if (current && await terminalPreferences.save(id === 'theme' ? { theme: getComputedStyle(document.documentElement).colorScheme === 'dark' ? 'light' : 'dark' } : { language: current.language === 'en' ? 'zh-CN' : 'en' })) toast.success(settingsText('saved')); }
  else if (id === 'clear') { new Set(transfers.snapshots.value.map(task => task.connectionId)).forEach(id => transfers.clearCompleted(id)); toast.info(paletteText('cleared')); }
  else if (workspace) await workspace.view(id === 'workspace' ? 'terminal' : id as 'monitor' | 'files');
}
const about = ref(false);
const editor = ref(false);
const editingId = ref<string | null>(null);
const manageGroups = ref(false);
const deleteTarget = ref<ServerProfile | null>(null);
const toast = useToast();
const selected = computed(() => servers.value.find(server => server.id === selectedId.value));
const backendState = computed(() => pending.value ? "pending" : error.value ? "error" : "ready");
const shortcut = computed(() => info.value?.platform === "windows" ? "Ctrl K" : "⌘ K");
const canManage = computed(() => !props.readOnly && !pending.value && !error.value);

function openEditor(id: string | null = null) {
  if (!canManage.value) return;
  editingId.value = id;
  editor.value = true;
}

async function onRemoved(message: string) {
  toast.success(message);
  selectServer(null);
  await nextTick();
  // The delete trigger may disappear with its server, so return to Home.
  document.querySelector<HTMLButtonElement>(".shell-brand")?.focus();
}

function selectServer(id: string | null) {
  pendingServerNavigation.value = null;
  selectedId.value = id;
}

async function revealWorkspaceView(serverId: string, view: "terminal" | "monitor" | "files") {
  selectedId.value = serverId;
  await nextTick();
  const snapshot = connections.snapshots.value[serverId];
  if (snapshot?.state !== "ready") return;
  const workspace = workspaceRefs.value.find(value => value.connectionId === snapshot.connectionId);
  await workspace?.view(view);
}

async function openServerView(server: ServerProfile, view: "terminal" | "monitor" | "files") {
  selectedId.value = server.id;
  const snapshot = connections.snapshots.value[server.id];
  if (snapshot?.state === "ready") {
    await revealWorkspaceView(server.id, view);
    return;
  }
  pendingServerNavigation.value = null;
  await connections.start(server);
  const next = connections.snapshots.value[server.id];
  if (next?.state === "ready") await revealWorkspaceView(server.id, view);
  else if (next && !isFinished(next)) pendingServerNavigation.value = { serverId: server.id, view };
}

function handleServerNavigation({ action, server }: ServerNavigationRequest) {
  if (action === "edit") {
    pendingServerNavigation.value = null;
    openEditor(server.id);
  } else if (action === "remove") {
    pendingServerNavigation.value = null;
    deleteTarget.value = { ...server };
  } else {
    const view = action === "workspace" ? "terminal" : action;
    void openServerView(server, view);
  }
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

function onConnectionTestResult(result: { kind: "success" | "error"; message: string }) {
  if (result.kind === "success") toast.success(result.message);
  else toast.error(result.message);
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
    await store.load();
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
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    paletteOpen.value = true;
  }
}
onMounted(() => { document.addEventListener("keydown", onKeydown); void load(); if (!props.readOnly) void terminalPreferences.load(); });
onBeforeUnmount(() => { document.removeEventListener("keydown", onKeydown); connections.dispose(); terminals.dispose(); transfers.dispose(); monitor.dispose(); });
</script>

<template>
  <div class="application-shell" :class="{ 'terminal-focused': focused }">
    <TopBar v-model:query="query" :home="!selected" :shortcut="shortcut" :can-manage="canManage" :settings-enabled="!readOnly" @home="selectServer(null)" @about="about = true" @add="openEditor()" @settings="settingsOpen = true" @palette="paletteOpen = true" />
    <div class="shell-content">
      <Sidebar v-model:query="query" :servers="servers" :groups="groups" :selected-id="selectedId" :pending="pending" :failed="!!error" :can-manage="canManage" @select="selectServer($event)" @add="openEditor()" @groups="manageGroups = true" @server-action="handleServerNavigation" />
      <main class="shell-main" :class="{ 'has-terminal-workspace': selectedWorkspace }" :aria-busy="pending">
        <BaseEmptyState v-if="pending" :title="t('loadingLocalData')" />
        <BaseEmptyState v-else-if="error" :title="t('localServiceUnavailable')" :description="error.message">
          <BaseButton @click="load">{{ t('retry') }}</BaseButton>
        </BaseEmptyState>
        <ServerOverview v-else-if="selected && !selectedWorkspace" :server="selected" :store="connections" :host-key-api="hostKeys" :network-api="network" :preflight-api="preflight" :read-only="!canManage" @back="selectServer(null)" @edit="openEditor($event)" @remove="deleteTarget = { ...$event }" @copy="copyOverviewValue" />
        <ServerList v-else-if="!selectedWorkspace && servers.length" :servers="filtered" :snapshots="connections.snapshots.value" :read-only="!canManage" :language="locale" @select="selectServer($event)" @edit="openEditor($event)" @remove="deleteTarget = { ...$event }" />
        <WelcomeView v-else-if="!selectedWorkspace" :has-servers="false" :can-manage="canManage" @about="about = true" @add="openEditor()" />
        <TerminalWorkspace v-for="[id, snapshot] in workspaces" ref="workspaceRefs" :key="snapshot.connectionId" v-show="!pending && !error && selectedId === id" :server="servers.find(server => server.id === id)!" :snapshot="snapshot" :controller="terminals" :sftp="sftp" :transfers="transfers" :monitor="monitor" :preferences="terminalPreferences" :background-images="backgroundImages" :visible="!pending && !error && selectedId === id" :busy="!!connections.busy.value[id]" :error="connections.errors.value[id]" @home="selectServer(null)" @disconnect="connections.disconnect(id, $event)" @focus-mode="focused = $event" @view="workspaceViews[snapshot.connectionId] = $event" />
      </main>
      <LocalBackendStatus :state="backendState" :version="info?.version" :terminal-count="terminals.tabs.value.length" :transfer-count="activeTransfers" />
    </div>
    <BaseDialog :open="about" :title="t('aboutMauLink')" @close="about = false">
      <p>{{ t('aboutLead') }}</p>
      <dl class="shell-about-details"><dt>{{ t('version') }}</dt><dd>{{ info?.version ?? '—' }}</dd><dt>{{ t('platform') }}</dt><dd>{{ info?.platform ?? '—' }}</dd><dt>{{ t('architecture') }}</dt><dd>{{ info?.architecture ?? '—' }}</dd></dl>
    </BaseDialog>
    <ServerDialog :language="locale" :open="editor" :server-id="editingId" :store="store" :connection-store="connections" @close="editor = false" @saved="toast.success($event)" @test-result="onConnectionTestResult" />
    <ConfirmDialog :language="locale" :server="deleteTarget" :store="store" @close="deleteTarget = null" @removed="onRemoved" />
    <GroupDialog :language="locale" :open="manageGroups" :store="store" @close="manageGroups = false" @saved="toast.success($event)" />
    <ConnectionDialogs :store="connections" :servers="servers" :suspended="settingsOpen || paletteOpen || about || (editor && !connections.draftTestActive.value) || manageGroups || !!deleteTarget" />
    <SettingsDialog :open="settingsOpen" :preferences="terminalPreferences" :background-images="backgroundImages" @close="settingsOpen = false" @saved="toast.success(settingsText('saved'))" />
    <CommandPalette :open="paletteOpen" :commands="commands" @close="paletteOpen = false" @execute="executeCommand" />
    <BaseToastViewport :queue="toast" />
  </div>
</template>
