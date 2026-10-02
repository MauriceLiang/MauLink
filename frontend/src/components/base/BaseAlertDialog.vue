<script setup lang="ts">
import BaseDialog from './BaseDialog.vue';
defineProps<{ open: boolean; title: string; busy?: boolean; closeLabel?: string; panelClass?: string; initialFocus?: string }>();
defineEmits<{ close: [] }>();
</script>
<template>
  <BaseDialog v-bind="$props" alert @close="$emit('close')"><slot /><template v-if="$slots.footer" #footer><slot name="footer" /></template></BaseDialog>
</template>
