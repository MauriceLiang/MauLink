<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerNavigationRequest } from "./server-navigation";
import BaseButton from "../components/base/BaseButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ServerNavigation from "./ServerNavigation.vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
defineProps<{ servers: ServerProfile[]; groups: Group[]; selectedId: string | null; query: string; pending: boolean; failed: boolean; canManage: boolean }>();
defineEmits<{ "update:query": [value: string]; select: [id: string]; add: []; groups: []; "server-action": [request: ServerNavigationRequest] }>();
</script>

<template>
  <aside class="shell-sidebar" :aria-label="t('serverWorkspace')">
    <div class="shell-sidebar-tools"><div class="shell-search shell-sidebar-search">
      <ShellIcon name="search" /><BaseInput :label="t('searchServers')" type="search" autocomplete="off" :model-value="query" :placeholder="t('search')" @update:model-value="$emit('update:query', $event)" />
    </div></div>
    <ServerNavigation :servers="servers" :groups="groups" :selected-id="selectedId" :query="query" :pending="pending" :failed="failed" :can-manage="canManage" @select="$emit('select', $event)" @groups="$emit('groups')" @action="$emit('server-action', $event)" />
    <div class="shell-sidebar-bottom"><BaseButton variant="softPrimary" size="lg" block :disabled="!canManage" @click="$emit('add')"><ShellIcon name="plus" />{{ t('addServer') }}</BaseButton></div>
  </aside>
</template>
