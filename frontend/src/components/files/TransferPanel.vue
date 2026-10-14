<script setup lang="ts">
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { computed } from "vue";
import type { TransferStore } from "../../stores/transfers";
import { transferFinished } from "../../stores/transfers";
import { formatSize, progress } from "../../files/path";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import BaseIconButton from "../base/BaseIconButton.vue";
const t = messages(filesMessages);
const props = defineProps<{ store: TransferStore; open: boolean; serverNames: Record<string, string> }>();
defineEmits<{ close: [] }>();
const tasks = computed(() => props.store.snapshots.value);
const connectionErrors = computed(() => Object.entries(props.store.errors.value).filter(([id, failure]) => failure !== null && !tasks.value.some(task => task.transferId === id)));
const completedConnections = computed(() => [...new Set(tasks.value.filter(task => task.state === 'completed').map(task => task.connectionId))]);
function serverName(connectionId: string) {
  return props.serverNames[connectionId] ?? t('unknownServerWithId', { id: connectionId.slice(0, 8) });
}
function clearCompleted() {
  completedConnections.value.forEach(connectionId => props.store.clearCompleted(connectionId));
}
const states = computed(() => ({ created: t('queued'), transferring: t('transferring'), finalizing: t('finalizing'), completed: t('completed'), cancelled: t('cancelled'), failed: t('failed') }));
</script>
<template>
  <aside v-show="open" id="global-transfer-drawer" class="global-transfer-drawer transfer-panel" aria-labelledby="transfer-drawer-title">
      <header><h2 id="transfer-drawer-title">{{ t('transfers') }}</h2><div class="transfer-drawer-actions"><BaseButton :disabled="!completedConnections.length" @click="clearCompleted">{{ t('clearCompleted') }}</BaseButton><BaseIconButton :label="t('closeTransfers')" @click="$emit('close')"><BaseIcon name="x" /></BaseIconButton></div></header>
      <div class="transfer-drawer-content">
        <p v-for="[connectionId, failure] in connectionErrors" :key="connectionId" role="alert">{{ serverName(connectionId) }}: {{ presentError(failure!).message }}</p>
        <p v-if="!tasks.length" class="files-empty">{{ t('emptyTransfers') }}</p>
        <ol><li v-for="task in tasks" :key="task.transferId" :data-state="task.state" :aria-label="`${task.fileName} ${states[task.state]}`">
          <div class="transfer-title"><strong :title="task.finalPath">{{ task.fileName }}</strong><span>{{ serverName(task.connectionId) }} · {{ task.direction === 'upload' ? t('upload') : t('download') }}</span></div>
          <progress :max="100" :value="task.state === 'completed' ? 100 : progress(task.totalBytes, task.transferredBytes) ?? (transferFinished(task) ? 0 : undefined)" :aria-label="t('progress', {name: task.fileName})" />
          <div class="transfer-details"><span :title="`${task.transferredBytes} / ${task.totalBytes ?? t('unknown')} bytes`">{{ formatSize(task.transferredBytes) }} / {{ formatSize(task.totalBytes) }}</span><span>{{ states[task.state] }}</span></div>
          <div class="transfer-extra">
            <p v-if="!transferFinished(task) && task.bytesPerSecond !== null">{{ formatSize(String(Math.round(task.bytesPerSecond))) }}/s<span v-if="task.remainingSeconds !== null"> · {{ t('remaining', {seconds: String(task.remainingSeconds)}) }}</span></p>
            <p v-if="task.error || store.errors.value[task.transferId]" role="alert">{{ presentError((task.error ?? store.errors.value[task.transferId])!).message }}</p>
            <p v-if="task.cleanupRequired" role="alert">{{ t('cleanupNote') }}<code>{{ task.temporaryPath ?? t('unknownPath') }}</code></p>
          </div>
          <BaseButton v-if="!transferFinished(task)" :disabled="store.cancelling.value.includes(task.transferId)" @click="store.cancel(task.transferId)">{{ store.cancelling.value.includes(task.transferId) ? t('cancelling') : t('cancelTransfer') }}</BaseButton>
        </li></ol>
      </div>
  </aside>
</template>
