<script setup lang="ts">
import { ToastProvider, ToastViewport } from 'reka-ui';
import { locale } from '../../i18n/locale';
import type { ToastQueue } from '../../composables/useToast';
import type { ToastPosition } from '../../../../contracts/v1/ToastPosition';
import BaseToast from './BaseToast.vue';
withDefaults(defineProps<{ queue: ToastQueue; position?: ToastPosition }>(), { position: 'topRight' });
</script>
<template>
  <ToastProvider :label="locale === 'en' ? 'Notification' : '通知'" disable-swipe>
    <BaseToast v-for="toast in queue.messages.value" :key="toast.id" :toast="toast" @close="queue.dismiss(toast.id)" @remove="queue.remove(toast.id)" />
    <Teleport to="body"><ToastViewport class="base-toast-viewport" :data-position="position" :label="locale === 'en' ? 'Notifications ({hotkey})' : '通知（{hotkey}）'" /></Teleport>
  </ToastProvider>
</template>
