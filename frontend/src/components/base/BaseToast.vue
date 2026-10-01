<script setup lang="ts">
import { locale } from "../../i18n/locale";
import BaseIconButton from "./BaseIconButton.vue";
withDefaults(defineProps<{ message: string; kind?: "success" | "error" | "info"; closeLabel?: string }>(), {
  kind: "info", closeLabel: "",
});
defineEmits<{ close: [] }>();
</script>

<template>
  <div class="base-toast" :class="`base-toast--${kind}`" :role="kind === 'error' ? 'alert' : 'status'">
    <span>{{ message }}</span>
    <BaseIconButton :label="closeLabel || (locale === 'en' ? 'Close toast' : '关闭提示')" @click="$emit('close')">×</BaseIconButton>
  </div>
</template>
