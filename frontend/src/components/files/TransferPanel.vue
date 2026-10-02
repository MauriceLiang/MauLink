<script setup lang="ts">
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { computed } from "vue";
import type { TransferStore } from "../../stores/transfers";
import { transferFinished } from "../../stores/transfers";
import { formatSize, progress } from "../../files/path";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
const t = messages(filesMessages);
const props = defineProps<{ store: TransferStore; connectionId: string }>();
const tasks = computed(() => props.store.snapshots.value.filter(value => value.connectionId === props.connectionId));
const states = computed(() => ({ created: t('queued'), transferring: t('transferring'), finalizing: t('finalizing'), completed: t('completed'), cancelled: t('cancelled'), failed: t('failed') }));
</script>
<template>
  <aside class="transfer-panel" :aria-label="t('transfers')">
    <header><h2>{{ t('transfers') }}</h2><BaseButton :disabled="!tasks.some(task => task.state === 'completed')" @click="store.clearCompleted(connectionId)">{{ t('clearCompleted') }}</BaseButton></header>
    <p v-if="store.errors.value[connectionId]" role="alert">{{ presentError(store.errors.value[connectionId]!).message }}</p>
    <p v-if="!tasks.length" class="files-empty">{{ t('emptyTransfers') }}</p>
    <ol><li v-for="task in tasks" :key="task.transferId" :data-state="task.state" :aria-label="`${task.fileName} ${states[task.state]}`">
      <div class="transfer-title"><strong :title="task.finalPath">{{ task.fileName }}</strong><span>{{ task.direction === 'upload' ? t('upload') : t('download') }}</span></div>
      <progress :max="100" :value="task.state === 'completed' ? 100 : progress(task.totalBytes, task.transferredBytes) ?? (transferFinished(task) ? 0 : undefined)" :aria-label="t('progress', {name: task.fileName})" />
      <div class="transfer-details"><span :title="`${task.transferredBytes} / ${task.totalBytes ?? t('unknown')} bytes`">{{ formatSize(task.transferredBytes) }} / {{ formatSize(task.totalBytes) }}</span><span>{{ states[task.state] }}</span></div>
      <p v-if="!transferFinished(task) && task.bytesPerSecond !== null">{{ formatSize(String(Math.round(task.bytesPerSecond))) }}/s<span v-if="task.remainingSeconds !== null">{{ t('remaining', {seconds: String(task.remainingSeconds)}) }}</span></p>
      <p v-if="task.error || store.errors.value[task.transferId]" role="alert">{{ presentError((task.error ?? store.errors.value[task.transferId])!).message }}</p>
      <p v-if="task.cleanupRequired" role="alert">{{ t('cleanupNote') }}<code>{{ task.temporaryPath ?? t('unknownPath') }}</code></p>
      <BaseButton v-if="!transferFinished(task)" :disabled="store.cancelling.value.includes(task.transferId)" @click="store.cancel(task.transferId)">{{ store.cancelling.value.includes(task.transferId) ? t('cancelling') : t('cancelTransfer') }}</BaseButton>
    </li></ol>
  </aside>
</template>
