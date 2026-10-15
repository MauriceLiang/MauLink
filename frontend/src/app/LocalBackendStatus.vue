<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { computed } from "vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
const props = defineProps<{ state: "pending" | "ready" | "error"; workspaceCount?: number; transferCount?: number; transferDrawerOpen?: boolean }>();
defineEmits<{ 'toggle-transfers': [] }>();
const label = computed(() => ({ pending: t('connectingLocal'), ready: t('readyLocal'), error: t('unavailableLocal') })[props.state]);
</script>

<template>
  <footer class="shell-statusbar">
    <span class="shell-backend-status" role="status"><span class="shell-backend-indicator" :data-state="state" aria-hidden="true"></span>{{ label }}</span>
    <span class="shell-statusbar-spacer"></span><button class="shell-transfer-status" type="button" :aria-expanded="transferDrawerOpen ?? false" aria-controls="global-transfer-drawer" @click="$emit('toggle-transfers')"><ShellIcon name="transfer" />{{ t(transferCount === 1 ? 'oneTransfer' : 'transferCount', { count: transferCount ?? 0 }) }}</button>
    <span class="shell-statusbar-separator"></span><span>{{ t(workspaceCount === 1 ? 'oneWorkspace' : 'workspaceCount', { count: workspaceCount ?? 0 }) }}</span>
  </footer>
</template>
