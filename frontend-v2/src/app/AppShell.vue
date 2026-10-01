<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { AppInfo } from "../ipc/commands";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { IpcClient } from "../ipc/client";
import { createTerminalApi } from "../ipc/terminal";
import { createSettingsApi } from "../ipc/settings";
import { createTerminalController, type TerminalChannelFactory } from "../terminal/controller";
import { createTerminalPreferences } from "../terminal/preferences";
import TerminalWorkspace from "../components/terminal/TerminalWorkspace.vue";
import { createConnectionApi } from "../ipc/connection";
import { createConnectionStore } from "../stores/connections";
import ConnectionPanel from "../components/connection/ConnectionPanel.vue";
import ConnectionDialogs from "../dialogs/ConnectionDialogs.vue";
import { createServerApi } from "../ipc/server";
import { createServerStore } from "../stores/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseEmptyState from "../components/base/BaseEmptyState.vue";
import BaseToast from "../components/base/BaseToast.vue";
import ServerList from "../components/server/ServerList.vue";
import ServerDialog from "../dialogs/ServerDialog.vue";
import ConfirmDialog from "../dialogs/ConfirmDialog.vue";
import GroupDialog from "../dialogs/GroupDialog.vue";
import TopBar from "./TopBar.vue";
import Sidebar from "./Sidebar.vue";
import LocalBackendStatus from "./LocalBackendStatus.vue";
import WelcomeView from "./WelcomeView.vue";

const props = withDefaults(defineProps<{ client: IpcClient; readOnly?: boolean; terminalChannelFactory?: TerminalChannelFactory }>(), { readOnly: false });
const info = ref<AppInfo | null>(null);
const store = createServerStore(createServerApi(props.client));
const { servers, groups, pending, error, query, filtered } = store;
const connections = createConnectionStore(createConnectionApi(props.client));
const terminals = createTerminalController(createTerminalApi(props.client), props.terminalChannelFactory);
const terminalPreferences = createTerminalPreferences(createSettingsApi(props.client), terminals.applySettings);
terminals.setCopyPreference(() => terminalPreferences.copyOnSelect.value);
const focused = ref(false);
const workspaces = computed(() => Object.entries(connections.snapshots.value).filter(([id, snapshot]) => servers.value.some(server => server.id === id) && (snapshot.state === 'ready' || terminals.tabs.value.some(tab => tab.connectionId === snapshot.connectionId))));
const selectedWorkspace = computed(() => workspaces.value.some(([id]) => id === selectedId.value));
const selectedId = ref<string | null>(null);
const about = ref(false);
const editor = ref(false);
const editingId = ref<string | null>(null);
const manageGroups = ref(false);
const deleteTarget = ref<ServerProfile | null>(null);
const notice = ref("");
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
  notice.value = message;
  selectedId.value = null;
  await nextTick();
  // The delete trigger may disappear with its server, so return to Home.
  document.querySelector<HTMLButtonElement>(".shell-brand")?.focus();
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
  if ((event.target instanceof Element && event.target.closest(".xterm")) || document.querySelector('[role="dialog"]') || connections.challenge.value || about.value || editor.value || manageGroups.value || deleteTarget.value) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    document.getElementById("shell-global-search")?.focus();
  }
}
onMounted(() => { document.addEventListener("keydown", onKeydown); void load(); });
onBeforeUnmount(() => { document.removeEventListener("keydown", onKeydown); connections.dispose(); terminals.dispose(); });
</script>

<template>
  <div class="application-shell" :class="{ 'terminal-focused': focused }">
    <TopBar v-model:query="query" :home="!selected" :shortcut="shortcut" :can-manage="canManage" @home="selectedId = null" @about="about = true" @add="openEditor()" />
    <div class="shell-content">
      <Sidebar v-model:query="query" :servers="servers" :groups="groups" :selected-id="selectedId" :pending="pending" :failed="!!error" :can-manage="canManage" @select="selectedId = $event" @add="openEditor()" @groups="manageGroups = true" />
      <main class="shell-main" :class="{ 'has-terminal-workspace': selectedWorkspace }" :aria-busy="pending">
        <BaseEmptyState v-if="pending" title="正在加载本地数据…" />
        <BaseEmptyState v-else-if="error" title="本地服务暂不可用" :description="error.message">
          <BaseButton @click="load">重试</BaseButton>
        </BaseEmptyState>
        <ConnectionPanel v-else-if="selected && !selectedWorkspace" :server="selected" :store="connections" :read-only="!canManage">
          <BaseButton @click="selectedId = null">返回服务器</BaseButton>
          <BaseButton :disabled="!canManage" @click="openEditor(selected.id)">编辑服务器</BaseButton>
          <BaseButton :disabled="!canManage" @click="deleteTarget = { ...selected }">删除服务器</BaseButton>
        </ConnectionPanel>
        <ServerList v-else-if="!selectedWorkspace && servers.length" :servers="filtered" :snapshots="connections.snapshots.value" :read-only="!canManage" @select="selectedId = $event" @edit="openEditor($event)" @remove="deleteTarget = { ...$event }" />
        <WelcomeView v-else-if="!selectedWorkspace" :has-servers="false" :can-manage="canManage" @about="about = true" @add="openEditor()" />
        <TerminalWorkspace v-for="[id, snapshot] in workspaces" :key="snapshot.connectionId" v-show="!pending && !error && selectedId === id" :server="servers.find(server => server.id === id)!" :snapshot="snapshot" :controller="terminals" :preferences="terminalPreferences" :visible="!pending && !error && selectedId === id" :busy="!!connections.busy.value[id]" :error="connections.errors.value[id]" @home="selectedId = null" @disconnect="connections.disconnect(id)" @focus-mode="focused = $event" />
      </main>
      <LocalBackendStatus :state="backendState" :version="info?.version" :terminal-count="terminals.tabs.value.length" />
    </div>
    <BaseDialog :open="about" title="关于 MauLink" @close="about = false">
      <p>安全、清晰的远程服务器工作台</p>
      <dl class="shell-about-details"><dt>版本</dt><dd>{{ info?.version ?? '—' }}</dd><dt>平台</dt><dd>{{ info?.platform ?? '—' }}</dd><dt>架构</dt><dd>{{ info?.architecture ?? '—' }}</dd></dl>
    </BaseDialog>
    <ServerDialog :open="editor" :server-id="editingId" :store="store" @close="editor = false" @saved="notice = $event" />
    <ConfirmDialog :server="deleteTarget" :store="store" @close="deleteTarget = null" @removed="onRemoved" />
    <GroupDialog :open="manageGroups" :store="store" @close="manageGroups = false" @saved="notice = $event" />
    <ConnectionDialogs :store="connections" :servers="servers" :suspended="about || editor || manageGroups || !!deleteTarget" />
    <BaseToast v-if="notice" class="server-notice" :message="notice" kind="info" @close="notice = ''" />
  </div>
</template>
