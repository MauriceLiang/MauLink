<script setup lang="ts">
withDefaults(defineProps<{
  disabled?: boolean;
  loading?: boolean;
  variant?: "primary" | "secondary" | "danger";
  type?: "button" | "submit" | "reset";
}>(), { disabled: false, loading: false, variant: "secondary", type: "button" });
</script>

<template>
  <button class="base-button" :class="`base-button--${variant}`" :type="type"
    :disabled="disabled || loading" :aria-busy="loading">
    <slot />
  </button>
</template>
