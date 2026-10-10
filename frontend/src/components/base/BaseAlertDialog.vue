<script setup lang="ts">
import BaseDialog from './BaseDialog.vue';
import type { DialogSize } from './dialog.types';
withDefaults(defineProps<{ open: boolean; title: string; busy?: boolean; closeLabel?: string; panelClass?: string; initialFocus?: string; size?: DialogSize }>(), { size: 'compact' });
defineEmits<{ close: []; closed: [] }>();
</script>
<template>
  <BaseDialog v-bind="$props" alert @close="$emit('close')" @closed="$emit('closed')"><slot /><template v-if="$slots.footer" #footer><slot name="footer" /></template></BaseDialog>
</template>
