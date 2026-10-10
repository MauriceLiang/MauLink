<script setup lang="ts">
import BaseIcon from "./BaseIcon.vue";
import type { ButtonSize, ButtonVariant } from "./button.types";

withDefaults(defineProps<{
  disabled?: boolean;
  loading?: boolean;
  variant?: ButtonVariant;
  size?: ButtonSize;
  block?: boolean;
  type?: "button" | "submit" | "reset";
}>(), { disabled: false, loading: false, variant: "secondary", size: "md", block: false, type: "button" });
</script>

<template>
  <button class="base-button" :class="[`base-button--${variant}`, `base-button--${size}`, {'base-button--block': block, 'base-button--loading': loading}]" :type="type"
    :disabled="disabled || loading" :aria-busy="loading">
    <span class="base-button-label"><slot /></span><BaseIcon v-if="loading" name="spinner" class="base-button-spinner" />
  </button>
</template>
