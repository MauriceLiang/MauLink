<script setup lang="ts">
// Paths from the existing Lucide icon set; see ../../LICENSE-lucide.
defineProps<{ name: "house" | "search" | "plus" | "settings" | "info" | "server" | "transfer" }>();
</script>

<template>
  <svg class="shell-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
    <template v-if="name === 'house'"><path d="M3 10.182 12 3l9 7.182V21a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" /><path d="M9 22v-9h6v9" /></template>
    <template v-else-if="name === 'search'"><path d="m21 21-4.34-4.34" /><circle cx="11" cy="11" r="8" /></template>
    <template v-else-if="name === 'plus'"><path d="M5 12h14M12 5v14" /></template>
    <template v-else-if="name === 'settings'"><path d="M14 17H5M19 7h-9" /><circle cx="17" cy="17" r="3" /><circle cx="7" cy="7" r="3" /></template>
    <template v-else-if="name === 'info'"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" /></template>
    <template v-else-if="name === 'server'"><rect width="20" height="8" x="2" y="2" rx="2" /><rect width="20" height="8" x="2" y="14" rx="2" /><path d="M6 6h.01M6 18h.01" /></template>
    <template v-else><path d="M12 16V4m-4 4 4-4 4 4M4 16v4h16v-4" /></template>
  </svg>
</template>
