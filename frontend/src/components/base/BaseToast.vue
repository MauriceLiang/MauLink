<script setup lang="ts">
import { ToastRoot, ToastTitle, ToastDescription, ToastAction, ToastClose } from 'reka-ui';
import type { ToastMessage } from '../../composables/useToast';
import { locale } from '../../i18n/locale';
import BaseIcon from './BaseIcon.vue';
import BaseButton from './BaseButton.vue';
defineProps<{ toast: ToastMessage }>();
defineEmits<{ close: []; remove: [] }>();
</script>
<template>
  <ToastRoot class="base-toast" :class="`base-toast--${toast.kind}`" :open="toast.open" :duration="toast.duration" :type="toast.kind === 'error' || toast.kind === 'warning' ? 'foreground' : 'background'" @update:open="!$event && $emit('close')">
    <div class="base-toast-body" @vue:unmounted="$emit('remove')"><BaseIcon :name="toast.kind" /><div><ToastTitle v-if="toast.title" class="base-toast-title">{{ toast.title }}</ToastTitle><ToastDescription>{{ toast.description }}</ToastDescription></div></div>
    <ToastAction v-if="toast.action" :alt-text="toast.action.label" as-child><BaseButton @click="toast.action.run()">{{ toast.action.label }}</BaseButton></ToastAction>
    <ToastClose class="base-toast-close" :aria-label="locale === 'en' ? 'Close toast' : '关闭提示'"><BaseIcon name="x" /></ToastClose>
  </ToastRoot>
</template>
