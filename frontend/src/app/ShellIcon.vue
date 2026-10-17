<script setup lang="ts">
import BaseIcon from "../components/base/BaseIcon.vue";

defineProps<{ name: "house" | "search" | "plus" | "settings" | "gear" | "info" | "server" | "transfer" }>();
const iconMap = { house: "house", search: "search", plus: "plus", settings: "settings", gear: "gear", info: "info", server: "server", transfer: "upload" } as const;
</script>

<template>
  <BaseIcon class="shell-icon" :name="iconMap[name]" />
</template>
