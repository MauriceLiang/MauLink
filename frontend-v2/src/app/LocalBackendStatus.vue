<script setup lang="ts">
import { computed } from "vue";
import ShellIcon from "./ShellIcon.vue";
const props = defineProps<{ state: "pending" | "ready" | "error"; version?: string; terminalCount?: number; transferCount?: number }>();
const label = computed(() => ({ pending: "正在连接本地服务", ready: "本地服务已就绪", error: "本地服务暂不可用" })[props.state]);
</script>

<template>
  <footer class="shell-statusbar" role="status">
    <span class="shell-backend-indicator" :data-state="state" aria-hidden="true"></span><span>{{ label }}</span>
    <span class="shell-statusbar-spacer"></span><span class="shell-transfer-status"><ShellIcon name="transfer" />{{ transferCount ?? 0 }} 项传输</span>
    <span class="shell-statusbar-separator"></span><span>{{ terminalCount ?? 0 }} 个终端</span>
    <span class="shell-statusbar-separator"></span><span>MauLink Desktop {{ version ? `v${version}` : '—' }}</span>
  </footer>
</template>
