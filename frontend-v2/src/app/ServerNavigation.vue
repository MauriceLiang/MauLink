<script setup lang="ts">
import { computed } from "vue";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import ShellIcon from "./ShellIcon.vue";
const props = defineProps<{ servers: ServerProfile[]; groups: Group[]; selectedId: string | null; query: string; pending: boolean; failed: boolean }>();
defineEmits<{ select: [id: string] }>();
const sections = computed(() => {
  const query = props.query.trim().toLocaleLowerCase();
  const items = props.servers.filter(server => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(query));
  const known = new Set(props.groups.map(group => group.id));
  return [...props.groups.map(group => ({ id: group.id, name: group.name, items: items.filter(server => server.groupId === group.id) })),
    { id: "", name: "未分组", items: items.filter(server => !server.groupId || !known.has(server.groupId)) }]
    .filter(section => section.items.length > 0);
});
</script>

<template>
  <nav class="shell-server-navigation" aria-label="按分组显示的服务器" aria-live="polite" :aria-busy="pending">
    <p v-if="pending" class="shell-nav-empty">正在加载服务器…</p>
    <p v-else-if="failed" class="shell-nav-empty">本地服务暂不可用</p>
    <p v-else-if="!sections.length" class="shell-nav-empty">{{ servers.length ? '没有找到匹配项' : '还没有服务器' }}</p>
    <template v-else>
      <div class="shell-group-heading"><span>分组</span><BaseIconButton class="shell-small-icon" label="新建分组" disabled><ShellIcon name="plus" /></BaseIconButton></div>
      <section v-for="section in sections" :key="section.id" class="shell-server-group" :aria-label="section.name">
        <h2 class="shell-group-title"><span>{{ section.name }}</span><span>{{ section.items.length }}</span></h2>
        <BaseButton v-for="server in section.items" :key="server.id" class="shell-server-item"
          :class="{ 'is-selected': server.id === selectedId }" :aria-current="server.id === selectedId ? 'page' : undefined"
          :aria-label="`${server.name} · ${server.host}`" @click="$emit('select', server.id)">
          <span class="shell-server-dot" aria-hidden="true"></span>
          <span class="shell-server-meta"><span class="shell-server-name">{{ server.name }}</span><span class="shell-server-host">{{ server.host }}</span></span>
        </BaseButton>
      </section>
    </template>
  </nav>
</template>
