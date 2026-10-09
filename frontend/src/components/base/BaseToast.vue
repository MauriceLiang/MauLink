<script setup lang="ts">
import { ToastRoot, ToastTitle, ToastDescription, ToastAction, ToastClose } from 'reka-ui';
import type { ToastKind, ToastMessage } from '../../composables/useToast';
import { messages } from '../../i18n/locale';
import { toastMessages } from '../../i18n/toast';
import BaseIcon from './BaseIcon.vue';
import BaseButton from './BaseButton.vue';
const t = messages(toastMessages);
const titleKeys: Record<ToastKind, keyof typeof toastMessages> = { success: 'success', info: 'info', warning: 'warning', error: 'error' };
defineProps<{ toast: ToastMessage }>();
defineEmits<{ close: []; remove: [] }>();
function defaultTitle(kind: ToastKind) { return t(titleKeys[kind]); }
</script>
<template>
  <ToastRoot class="base-toast" :class="`base-toast--${toast.kind}`" :open="toast.open" :duration="toast.duration" :type="toast.kind === 'error' || toast.kind === 'warning' ? 'foreground' : 'background'" @update:open="!$event && $emit('close')">
    <BaseIcon class="base-toast-status-icon" :name="toast.kind" />
    <div class="base-toast-content" @vue:unmounted="$emit('remove')">
      <ToastTitle class="base-toast-title">{{ toast.title || defaultTitle(toast.kind) }}</ToastTitle>
      <ToastDescription class="base-toast-description">{{ toast.description }}</ToastDescription>
      <ToastAction v-if="toast.action" class="base-toast-action-wrapper" :alt-text="toast.action.label" as-child>
        <BaseButton class="base-toast-action" @click="toast.action.run()">{{ toast.action.label }}</BaseButton>
      </ToastAction>
    </div>
    <ToastClose class="base-toast-close" :aria-label="t('close')"><BaseIcon name="x" /></ToastClose>
  </ToastRoot>
</template>
