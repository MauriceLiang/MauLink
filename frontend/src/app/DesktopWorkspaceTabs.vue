<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import type { ConnectionState } from "../../../contracts/v1/ConnectionState";
import type { WorkspacePhase } from "../stores/workspace-ui";
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { connectionStateText } from "../i18n/connections";
import BaseIcon from "../components/base/BaseIcon.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";

const t = messages(shellMessages);
export interface WorkspaceTabItem {
  key: string;
  serverId: string;
  name: string;
  host: string;
  phase: WorkspacePhase;
  connectionState: ConnectionState | null;
  busy: boolean;
}
const props = defineProps<{ tabs: WorkspaceTabItem[]; activeKey: string | null }>();
const emit = defineEmits<{ select: [serverId: string]; close: [serverId: string] }>();
const tabButtons = new Map<string, HTMLButtonElement>();
const pendingFocus = ref<string | null>(null);

function setTabButton(serverId: string, element: unknown) {
  if (element instanceof HTMLButtonElement) tabButtons.set(serverId, element);
  else tabButtons.delete(serverId);
}

function tabState(tab: WorkspaceTabItem) {
  if (tab.connectionState) return connectionStateText(tab.connectionState);
  return tab.phase === "failed" ? t("connectionFailed") : t("connectingWorkspace");
}

async function focusTab(serverId: string) {
  emit("select", serverId);
  await nextTick();
  tabButtons.get(serverId)?.focus();
}

function onTabKeydown(event: KeyboardEvent, index: number) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  if (!props.tabs.length) return;
  event.preventDefault();
  const nextIndex = event.key === "Home" ? 0
    : event.key === "End" ? props.tabs.length - 1
      : Math.max(0, Math.min(props.tabs.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)));
  const next = props.tabs[nextIndex];
  if (next) void focusTab(next.serverId);
}

function closeTab(serverId: string) {
  pendingFocus.value = serverId;
  emit("close", serverId);
}

watch(() => props.tabs.map(tab => tab.key), async keys => {
  const closing = pendingFocus.value;
  if (!closing || keys.includes(closing)) return;
  pendingFocus.value = null;
  await nextTick();
  const target = props.tabs.find(tab => tab.serverId === props.activeKey) ?? props.tabs.at(-1);
  if (target) tabButtons.get(target.serverId)?.focus();
});
</script>

<template>
  <div class="shell-workspace-tabs" role="tablist" :aria-label="t('openWorkspaces')">
    <div v-for="(tab, index) in tabs" :key="tab.key" class="shell-workspace-tab" :class="{ 'is-active': activeKey === tab.serverId }">
      <button :ref="element => setTabButton(tab.serverId, element)"
        class="shell-workspace-tab__select" role="tab" :aria-selected="activeKey === tab.serverId"
        :aria-label="`${tab.name} · ${tabState(tab)}`" :tabindex="activeKey === tab.serverId ? 0 : -1"
        @click="emit('select', tab.serverId)" @keydown="onTabKeydown($event, index)">
        <span class="shell-workspace-tab__state" :data-phase="tab.phase" aria-hidden="true"></span>
        <span class="shell-workspace-tab__name">{{ tab.name }}</span>
      </button>
      <BaseIconButton class="shell-workspace-tab__close" :label="t('closeWorkspace', { name: tab.name })" :disabled="tab.busy" @click.stop="closeTab(tab.serverId)">
        <BaseIcon name="x" />
      </BaseIconButton>
    </div>
  </div>
</template>
