<script setup lang="ts">
import BaseButton from '../base/BaseButton.vue';
import BaseDialog from '../base/BaseDialog.vue';
import { messages } from '../../i18n/locale';
import { updaterMessages } from '../../i18n/updater';

defineProps<{ open: boolean; busy?: boolean; activeConnections: number; activeTerminals: number }>();
const emit = defineEmits<{ cancel: []; confirm: [] }>();
const t = messages(updaterMessages);
</script>

<template>
  <BaseDialog :open="open" :busy="busy" :title="t('confirmTitle')" :alert="true" size="compact" @close="emit('cancel')">
    <p>{{ t('confirmDescription') }}</p>
    <ul class="update-install-confirm__activity">
      <li v-if="activeConnections">{{ t('activeConnections', { count: activeConnections }) }}</li>
      <li v-if="activeTerminals">{{ t('activeTerminals', { count: activeTerminals }) }}</li>
    </ul>
    <template #footer>
      <BaseButton variant="secondary" data-dialog-cancel :disabled="busy" @click="emit('cancel')">{{ t('continueWorking') }}</BaseButton>
      <BaseButton variant="primary" :disabled="busy" @click="emit('confirm')">{{ t('disconnectAndInstall') }}</BaseButton>
    </template>
  </BaseDialog>
</template>
