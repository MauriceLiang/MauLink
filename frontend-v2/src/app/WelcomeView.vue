<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import BaseButton from "../components/base/BaseButton.vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
defineProps<{ hasServers: boolean; canManage: boolean }>();
defineEmits<{ about: []; add: [] }>();
</script>

<template>
  <section class="shell-welcome" aria-labelledby="shell-welcome-title">
    <div class="shell-welcome-icon" aria-hidden="true"><ShellIcon name="server" /></div>
    <h1 id="shell-welcome-title">{{ hasServers ? t('selectAServer') : t('noServersYet') }}</h1>
    <p>{{ hasServers ? t('selectLead') : t('welcomeLead') }}</p>
    <BaseButton v-if="!hasServers" variant="primary" :disabled="!canManage" @click="$emit('add')"><ShellIcon name="plus" />{{ t('addServer') }}</BaseButton>
    <button class="shell-learn-link" type="button" @click="$emit('about')">{{ t('learnAboutMauLink') }}</button>
  </section>
</template>
