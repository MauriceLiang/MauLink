<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { computed } from "vue";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerNavigationAction, ServerNavigationRequest } from "./server-navigation";
import { paletteMessages } from "../i18n/palette";
import BaseContextMenu from "../components/base/BaseContextMenu.vue";
import type { MenuItem } from "../components/base/menu";
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
const paletteText = messages(paletteMessages);
const props = defineProps<{ servers: ServerProfile[]; groups: Group[]; selectedId: string | null; query: string; pending: boolean; failed: boolean; canManage: boolean }>();
const emit = defineEmits<{ select: [id: string]; groups: []; action: [request: ServerNavigationRequest] }>();
const serverActions = new Set<ServerNavigationAction>(["workspace", "monitor", "files", "edit", "remove"]);
function menuItems(): MenuItem[] {
  return [
    { id: "workspace", label: paletteText("workspace") },
    { id: "monitor", label: paletteText("monitor") },
    { id: "files", label: paletteText("files") },
    { id: "edit", label: t("editServer"), disabled: !props.canManage },
    { id: "remove", label: t("deleteServer"), danger: true, disabled: !props.canManage },
  ];
}
function selectAction(server: ServerProfile, action: string) {
  if (serverActions.has(action as ServerNavigationAction)) emit("action", { action: action as ServerNavigationAction, server });
}
const sections = computed(() => {
  const query = props.query.trim().toLocaleLowerCase();
  const items = props.servers.filter(server => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(query));
  const known = new Set(props.groups.map(group => group.id));
  return [...props.groups.map(group => ({ id: group.id, name: group.name, items: items.filter(server => server.groupId === group.id) })),
    { id: "", name: t('ungrouped'), items: items.filter(server => !server.groupId || !known.has(server.groupId)) }]
    .filter(section => section.items.length > 0);
});
</script>

<template>
  <nav class="shell-server-navigation" :aria-label="t('serversByGroup')" aria-live="polite" :aria-busy="pending">
    <div class="shell-group-heading"><span>{{ t('group') }}</span><BaseIconButton class="shell-small-icon" :label="t('manageGroups')" :disabled="!canManage" @click="$emit('groups')"><ShellIcon name="plus" /></BaseIconButton></div>
    <p v-if="pending" class="shell-nav-empty">{{ t('loadingServers') }}</p>
    <p v-else-if="failed" class="shell-nav-empty">{{ t('localServiceUnavailable') }}</p>
    <p v-else-if="!sections.length" class="shell-nav-empty">{{ servers.length ? t('noMatchesFound') : t('noServersYet') }}</p>
    <template v-else>
      <section v-for="section in sections" :key="section.id" class="shell-server-group" :aria-label="section.name">
        <h2 class="shell-group-title"><span>{{ section.name }}</span><span>{{ section.items.length }}</span></h2>
        <BaseContextMenu v-for="server in section.items" :key="server.id" :label="server.name" :items="menuItems()" @action="selectAction(server, $event)">
          <BaseButton class="shell-server-item"
            :class="{ 'is-selected': server.id === selectedId }" :aria-current="server.id === selectedId ? 'page' : undefined"
            :aria-label="`${server.name} · ${server.host}`" @click="emit('select', server.id)">
            <span class="shell-server-dot" aria-hidden="true"></span>
            <span class="shell-server-meta"><span class="shell-server-name">{{ server.name }}</span><span class="shell-server-host">{{ server.host }}</span></span>
          </BaseButton>
        </BaseContextMenu>
      </section>
    </template>
  </nav>
</template>
