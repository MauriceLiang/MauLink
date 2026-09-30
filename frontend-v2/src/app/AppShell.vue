<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { AppInfo } from "../ipc/commands";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { IpcClient } from "../ipc/client";
import { createServerApi } from "../ipc/server";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseEmptyState from "../components/base/BaseEmptyState.vue";
import TopBar from "./TopBar.vue";
import Sidebar from "./Sidebar.vue";
import LocalBackendStatus from "./LocalBackendStatus.vue";
import WelcomeView from "./WelcomeView.vue";

const props = defineProps<{ client: IpcClient }>();
const info = ref<AppInfo | null>(null);
const servers = ref<ServerProfile[]>([]);
const groups = ref<Group[]>([]);
const pending = ref(true);
const error = ref<ReturnType<typeof presentError> | null>(null);
const query = ref("");
const selectedId = ref<string | null>(null);
const about = ref(false);
const selected = computed(() => servers.value.find(server => server.id === selectedId.value));
const backendState = computed(() => pending.value ? "pending" : error.value ? "error" : "ready");
const shortcut = computed(() => info.value?.platform === "windows" ? "Ctrl K" : "⌘ K");

async function load() {
  pending.value = true;
  error.value = null;
  try {
    info.value = await props.client.getInfo();
    const api = createServerApi(props.client);
    groups.value = await api.listGroups();
    const items: ServerProfile[] = [];
    let cursor: string | null = null;
    do {
      const page = await api.list({ query: null, groupId: null, limit: 200, cursor });
      items.push(...page.items);
      cursor = page.nextCursor;
    } while (cursor);
    servers.value = items;
  } catch (failure) {
    error.value = presentError(mapError(failure));
  } finally {
    pending.value = false;
  }
}

function onKeydown(event: KeyboardEvent) {
  if (about.value) return;
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
    <TopBar v-model:query="query" :home="!selected" :shortcut="shortcut" @home="selectedId = null" @about="about = true" />
    <div class="shell-content">
      <Sidebar v-model:query="query" :servers="servers" :groups="groups" :selected-id="selectedId" :pending="pending" :failed="!!error" @select="selectedId = $event" />
      <main class="shell-main" :aria-busy="pending">
        <BaseEmptyState v-if="pending" title="正在加载本地数据…" />
        <BaseEmptyState v-else-if="error" title="本地服务暂不可用" :description="error.message">
          <BaseButton @click="load">重试</BaseButton>
        </BaseEmptyState>
        <BaseEmptyState v-else-if="selected" :title="selected.name" :description="`${selected.username}@${selected.host}:${selected.port} · 尚未连接`">
          <BaseButton @click="selectedId = null">返回服务器</BaseButton>
        </BaseEmptyState>
        <WelcomeView v-else :has-servers="servers.length > 0" @about="about = true" />
      </main>
      <LocalBackendStatus :state="backendState" :version="info?.version" />
    </div>
    <BaseDialog :open="about" title="关于 MauLink" @close="about = false">
      <p>安全、清晰的远程服务器工作台</p>
      <dl class="shell-about-details"><dt>版本</dt><dd>{{ info?.version ?? '—' }}</dd><dt>平台</dt><dd>{{ info?.platform ?? '—' }}</dd><dt>架构</dt><dd>{{ info?.architecture ?? '—' }}</dd></dl>
    </BaseDialog>
  </div>
</template>
