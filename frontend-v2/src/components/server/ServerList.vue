<script setup lang="ts">
import type { Language } from "../../../../contracts/v1/Language";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { serverText, type ServerMessage } from "../../i18n/servers";
import ServerCard from "./ServerCard.vue";
import BaseEmptyState from "../base/BaseEmptyState.vue";
const props = withDefaults(defineProps<{ servers: ServerProfile[]; language?: Language; readOnly?: boolean }>(), { language: "zh-CN", readOnly: false });
defineEmits<{ select: [id: string]; edit: [id: string]; remove: [server: ServerProfile] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
</script>

<template>
  <section class="server-home" :aria-label="t('servers')"><h1>{{ t('servers') }}</h1><p class="server-home-lead">{{ t('lead') }}</p>
    <div v-if="servers.length" class="server-home-list"><ServerCard v-for="server in servers" :key="server.id" :server="server" :language="language" :read-only="readOnly" @select="$emit('select', $event)" @edit="$emit('edit', $event)" @remove="$emit('remove', $event)" /></div>
    <BaseEmptyState v-else :title="t('empty')" />
  </section>
</template>
