<script setup lang="ts">
import { computed, ref } from "vue";
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ConnectionSnapshot } from "../../../contracts/v1/ConnectionSnapshot";
import type { ServerNavigationAction, ServerNavigationRequest } from "./server-navigation";
import { paletteMessages } from "../i18n/palette";
import { connectionStateText } from "../i18n/connections";
import BaseContextMenu from "../components/base/BaseContextMenu.vue";
import type { MenuItem } from "../components/base/menu";
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import ShellIcon from "./ShellIcon.vue";

const t = messages(shellMessages);
const paletteText = messages(paletteMessages);
const props = defineProps<{ servers: ServerProfile[]; groups: Group[]; snapshots: Record<string, ConnectionSnapshot>; selectedId: string | null; query: string; pending: boolean; failed: boolean; canManage: boolean }>();
const emit = defineEmits<{ select: [id: string]; groups: []; action: [request: ServerNavigationRequest] }>();
const navigation = ref<HTMLElement | null>(null);
const collapsedGroups = ref(new Set<string>());
const serverActions = new Set<ServerNavigationAction>(["workspace", "monitor", "files", "details", "edit", "remove"]);

function menuItems(): MenuItem[] {
  return [
    { id: "workspace", label: paletteText("workspace") },
    { id: "monitor", label: paletteText("monitor") },
    { id: "files", label: paletteText("files") },
    { id: "details", label: t("serverDetails") },
    { id: "edit", label: t("editServer"), disabled: !props.canManage },
    { id: "remove", label: t("deleteServer"), danger: true, disabled: !props.canManage },
  ];
}

function selectAction(server: ServerProfile, action: string) {
  if (serverActions.has(action as ServerNavigationAction)) emit("action", { action: action as ServerNavigationAction, server });
}

function toggleGroup(id: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(id)) next.delete(id); else next.add(id);
  collapsedGroups.value = next;
}

function isCollapsed(id: string) { return collapsedGroups.value.has(id); }

function connectionState(serverId: string) {
  const state = props.snapshots[serverId]?.state;
  if (state === "ready") return "ready";
  if (state === "failed") return "failed";
  if (state && !["closed", "cancelled"].includes(state)) return "connecting";
  return "disconnected";
}

function connectionLabel(serverId: string) {
  const snapshot = props.snapshots[serverId];
  return snapshot ? connectionStateText(snapshot.state) : t("notConnected");
}

const sections = computed(() => {
  const query = props.query.trim().toLocaleLowerCase();
  const items = props.servers.filter(server => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(query));
  const known = new Set(props.groups.map(group => group.id));
  return [...props.groups.map(group => ({ id: group.id, name: group.name, items: items.filter(server => server.groupId === group.id) })),
    { id: "", name: t("ungrouped"), items: items.filter(server => !server.groupId || !known.has(server.groupId)) }]
    .filter(section => section.items.length > 0);
});

function onTreeKeydown(event: KeyboardEvent) {
  const target = event.currentTarget as HTMLElement;
  const groupId = target.dataset.groupId;
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    const items = [...(navigation.value?.querySelectorAll<HTMLElement>("[data-tree-item]") ?? [])];
    const index = items.indexOf(target);
    const next = items[index + (event.key === "ArrowDown" ? 1 : -1)];
    if (next) { event.preventDefault(); next.focus(); }
  } else if (groupId !== undefined && event.key === "ArrowLeft" && !isCollapsed(groupId)) {
    event.preventDefault(); toggleGroup(groupId);
  } else if (groupId !== undefined && event.key === "ArrowRight") {
    if (isCollapsed(groupId)) { event.preventDefault(); toggleGroup(groupId); }
    else {
      const firstServer = target.closest(".shell-server-group")?.querySelector<HTMLElement>(".shell-server-item");
      if (firstServer) { event.preventDefault(); firstServer.focus(); }
    }
  } else if (groupId === undefined && event.key === "ArrowLeft") {
    const heading = target.closest(".shell-server-group")?.querySelector<HTMLElement>(".shell-group-title");
    if (heading) { event.preventDefault(); heading.focus(); }
  }
}
</script>

<template>
  <nav ref="navigation" class="shell-server-navigation" :aria-label="t('serversByGroup')" aria-live="polite" :aria-busy="pending" role="tree">
    <div v-if="servers.length" class="shell-group-heading"><span>{{ t('group') }}</span><BaseIconButton class="shell-small-icon" :label="t('manageGroups')" :disabled="!canManage" @click="$emit('groups')"><ShellIcon name="plus" /></BaseIconButton></div>
    <p v-if="pending" class="shell-nav-empty">{{ t('loadingServers') }}</p>
    <p v-else-if="failed" class="shell-nav-empty">{{ t('localServiceUnavailable') }}</p>
    <p v-else-if="!sections.length" class="shell-nav-empty">{{ servers.length ? t('noMatchesFound') : t('noServersYet') }}</p>
    <template v-else>
      <section v-for="section in sections" :key="section.id || 'ungrouped'" class="shell-server-group" role="group" :aria-label="section.name">
        <BaseButton class="shell-group-title" variant="ghost" role="treeitem" :aria-expanded="!isCollapsed(section.id)" :data-group-id="section.id" data-tree-item @click="toggleGroup(section.id)" @keydown="onTreeKeydown">
          <span>{{ section.name }}</span><span>{{ section.items.length }}</span><BaseIcon :class="{ 'is-collapsed': isCollapsed(section.id) }" name="chevron-down" />
        </BaseButton>
        <template v-if="!isCollapsed(section.id)">
          <BaseContextMenu v-for="server in section.items" :key="server.id" :label="server.name" :items="menuItems()" @action="selectAction(server, $event)">
            <BaseButton class="shell-server-item" role="treeitem" :class="{ 'is-selected': server.id === selectedId }" :aria-current="server.id === selectedId ? 'page' : undefined"
              :aria-label="`${server.name} · ${server.host} · ${connectionLabel(server.id)}`" :data-connection="connectionState(server.id)" data-tree-item @click="emit('select', server.id)" @keydown="onTreeKeydown">
              <span class="shell-server-dot" :data-state="connectionState(server.id)" aria-hidden="true"></span>
              <span class="shell-server-meta"><span class="shell-server-name">{{ server.name }}</span><span class="shell-server-host">{{ server.host }}</span></span>
            </BaseButton>
          </BaseContextMenu>
        </template>
      </section>
    </template>
  </nav>
</template>
