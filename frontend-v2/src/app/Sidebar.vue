<script setup lang="ts">
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import BaseButton from "../components/base/BaseButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ServerNavigation from "./ServerNavigation.vue";
import ShellIcon from "./ShellIcon.vue";
defineProps<{ servers: ServerProfile[]; groups: Group[]; selectedId: string | null; query: string; pending: boolean; failed: boolean; canManage: boolean }>();
defineEmits<{ "update:query": [value: string]; select: [id: string]; add: []; groups: [] }>();
</script>

<template>
  <aside class="shell-sidebar" aria-label="服务器工作区">
    <div class="shell-sidebar-tools"><div class="shell-search shell-sidebar-search">
      <ShellIcon name="search" /><BaseInput label="搜索服务器" type="search" autocomplete="off" :model-value="query" placeholder="搜索" @update:model-value="$emit('update:query', $event)" />
    </div></div>
    <ServerNavigation :servers="servers" :groups="groups" :selected-id="selectedId" :query="query" :pending="pending" :failed="failed" :can-manage="canManage" @select="$emit('select', $event)" @groups="$emit('groups')" />
    <div class="shell-sidebar-bottom"><BaseButton :disabled="!canManage" @click="$emit('add')"><ShellIcon name="plus" />添加服务器</BaseButton></div>
  </aside>
</template>
