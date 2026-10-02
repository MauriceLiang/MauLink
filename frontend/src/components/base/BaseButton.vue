<script setup lang="ts">
import BaseIcon from "./BaseIcon.vue";
withDefaults(defineProps<{
  disabled?: boolean;
  loading?: boolean;
  variant?: "primary" | "secondary" | "danger";
  type?: "button" | "submit" | "reset";
}>(), { disabled: false, loading: false, variant: "secondary", type: "button" });
</script>

<template>
  <button class="base-button" :class="[`base-button--${variant}`, {'base-button--loading': loading}]" :type="type"
    :disabled="disabled || loading" :aria-busy="loading">
    <span class="base-button-label"><slot /></span><BaseIcon v-if="loading" name="spinner" class="base-button-spinner" />
  </button>
</template>
