<script setup lang="ts">
import BaseIconButton from "./BaseIconButton.vue";
withDefaults(defineProps<{ message: string; kind?: "success" | "error" | "info"; closeLabel?: string }>(), {
  kind: "info", closeLabel: "关闭提示",
});
defineEmits<{ close: [] }>();
</script>

<template>
  <div class="base-toast" :class="`base-toast--${kind}`" :role="kind === 'error' ? 'alert' : 'status'">
    <span>{{ message }}</span>
    <BaseIconButton :label="closeLabel" @click="$emit('close')">×</BaseIconButton>
  </div>
</template>
