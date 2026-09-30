<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import type { AppInfo } from "../ipc/commands";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { IpcClient } from "../ipc/client";
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

const props = withDefaults(defineProps<{ client: IpcClient; readOnly?: boolean }>(), { readOnly: false });
const info = ref<AppInfo | null>(null);
const store = createServerStore(createServerApi(props.client));
const { servers, groups, pending, error, query, filtered } = store;
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

function onKeydown(event: KeyboardEvent) {
  if (about.value || editor.value || manageGroups.value || deleteTarget.value) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    document.getElementById("shell-global-search")?.focus();
  }
}
onMounted(() => { document.addEventListener("keydown", onKeydown); void load(); });
onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="application-shell">
    <TopBar v-model:query="query" :home="!selected" :shortcut="shortcut" :can-manage="canManage" @home="selectedId = null" @about="about = true" @add="openEditor()" />
    <div class="shell-content">
      <Sidebar v-model:query="query" :servers="servers" :groups="groups" :selected-id="selectedId" :pending="pending" :failed="!!error" :can-manage="canManage" @select="selectedId = $event" @add="openEditor()" @groups="manageGroups = true" />
      <main class="shell-main" :aria-busy="pending">
        <BaseEmptyState v-if="pending" title="正在加载本地数据…" />
        <BaseEmptyState v-else-if="error" title="本地服务暂不可用" :description="error.message">
          <BaseButton @click="load">重试</BaseButton>
        </BaseEmptyState>
        <BaseEmptyState v-else-if="selected" :title="selected.name" :description="`${selected.username}@${selected.host}:${selected.port} · 尚未连接`">
          <BaseButton @click="selectedId = null">返回服务器</BaseButton>
          <BaseButton :disabled="!canManage" @click="openEditor(selected.id)">编辑服务器</BaseButton>
          <BaseButton :disabled="!canManage" @click="deleteTarget = { ...selected }">删除服务器</BaseButton>
        </BaseEmptyState>
        <ServerList v-else-if="servers.length" :servers="filtered" :read-only="!canManage" @select="selectedId = $event" @edit="openEditor($event)" @remove="deleteTarget = { ...$event }" />
        <WelcomeView v-else :has-servers="false" :can-manage="canManage" @about="about = true" @add="openEditor()" />
      </main>
      <LocalBackendStatus :state="backendState" :version="info?.version" />
    </div>
    <BaseDialog :open="about" title="关于 MauLink" @close="about = false">
      <p>安全、清晰的远程服务器工作台</p>
      <dl class="shell-about-details"><dt>版本</dt><dd>{{ info?.version ?? '—' }}</dd><dt>平台</dt><dd>{{ info?.platform ?? '—' }}</dd><dt>架构</dt><dd>{{ info?.architecture ?? '—' }}</dd></dl>
    </BaseDialog>
    <ServerDialog :open="editor" :server-id="editingId" :store="store" @close="editor = false" @saved="notice = $event" />
    <ConfirmDialog :server="deleteTarget" :store="store" @close="deleteTarget = null" @removed="onRemoved" />
    <GroupDialog :open="manageGroups" :store="store" @close="manageGroups = false" @saved="notice = $event" />
    <BaseToast v-if="notice" class="server-notice" :message="notice" kind="info" @close="notice = ''" />
  </div>
</template>
