<script setup lang="ts">
withDefaults(defineProps<{ tone?: "neutral" | "success" | "error" }>(), { tone: "neutral" });
</script>

<template>
  <span class="base-status-badge" :class="`base-status-badge--${tone}`"><slot /></span>
</template>
