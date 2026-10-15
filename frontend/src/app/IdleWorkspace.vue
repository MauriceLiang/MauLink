<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import BaseButton from "../components/base/BaseButton.vue";

const t = messages(shellMessages);
defineProps<{ hasServers: boolean }>();
defineEmits<{ add: [] }>();
</script>

<template>
  <section class="shell-welcome" :class="{ 'shell-welcome--no-servers': !hasServers }" aria-labelledby="shell-welcome-title">
    <template v-if="hasServers">
      <h1 id="shell-welcome-title">{{ t("noOpenSessions") }}</h1>
      <p>{{ t("selectServerToStart") }}</p>
    </template>
    <template v-else>
      <h1 id="shell-welcome-title">{{ t("noServersYet") }}</h1>
      <p>{{ t("addFirstServerLead") }}</p>
      <BaseButton variant="neutralPrimary" @click="$emit('add')">{{ t("addServer") }}</BaseButton>
    </template>
  </section>
</template>
