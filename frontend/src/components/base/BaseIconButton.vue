<script setup lang="ts">
import BaseButton from "./BaseButton.vue";
defineProps<{ label: string; disabled?: boolean }>();
</script>

<template>
  <BaseButton class="base-icon-button" :aria-label="label" :title="label" :disabled="disabled">
    <span aria-hidden="true"><slot /></span>
  </BaseButton>
</template>
