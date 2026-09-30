<script setup lang="ts">
import type { Language } from "../../../../contracts/v1/Language";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { serverText, type ServerMessage } from "../../i18n/servers";
import BaseButton from "../base/BaseButton.vue";
const props = withDefaults(defineProps<{ server: ServerProfile; language?: Language; readOnly?: boolean }>(), { language: "zh-CN", readOnly: false });
defineEmits<{ select: [id: string]; edit: [id: string]; remove: [server: ServerProfile] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
</script>

<template>
  <article class="server-home-card" :aria-label="server.name">
    <span class="server-home-dot" role="img" :aria-label="t('notConnected')"></span>
    <div class="server-home-details"><strong>{{ server.name }}</strong><span>{{ server.username }}@{{ server.host }}</span></div>
    <BaseButton :aria-label="`${t('view')} ${server.name}`" @click="$emit('select', server.id)">{{ t('view') }}</BaseButton>
    <details class="server-card-menu"><summary :aria-label="`${t('more')} ${server.name}`">⋯</summary><div>
      <BaseButton :disabled="readOnly" :aria-label="`${t('edit')} ${server.name}`" @click="$emit('edit', server.id)">{{ t('edit') }}</BaseButton>
      <BaseButton :disabled="readOnly" :aria-label="`${t('remove')} ${server.name}`" @click="$emit('remove', server)">{{ t('remove') }}</BaseButton>
    </div></details>
  </article>
</template>
