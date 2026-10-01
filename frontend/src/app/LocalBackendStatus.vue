<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import { computed } from "vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
const props = defineProps<{ state: "pending" | "ready" | "error"; version?: string; terminalCount?: number; transferCount?: number }>();
const label = computed(() => ({ pending: t('connectingLocal'), ready: t('readyLocal'), error: t('unavailableLocal') })[props.state]);
</script>

<template>
  <footer class="shell-statusbar" role="status">
    <span class="shell-backend-indicator" :data-state="state" aria-hidden="true"></span><span>{{ label }}</span>
    <span class="shell-statusbar-spacer"></span><span class="shell-transfer-status"><ShellIcon name="transfer" />{{ t(transferCount === 1 ? 'oneTransfer' : 'transferCount', { count: transferCount ?? 0 }) }}</span>
    <span class="shell-statusbar-separator"></span><span>{{ t(terminalCount === 1 ? 'oneTerminal' : 'terminalCount', { count: terminalCount ?? 0 }) }}</span>
    <span class="shell-statusbar-separator"></span><span>MauLink Desktop {{ version ? `v${version}` : '—' }}</span>
  </footer>
</template>
