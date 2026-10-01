<script setup lang="ts">
import { computed } from "vue";
import type { TransferStore } from "../../stores/transfers";
import { transferFinished } from "../../stores/transfers";
import { formatSize, progress } from "../../files/path";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
const props = defineProps<{ store: TransferStore; connectionId: string }>();
const tasks = computed(() => props.store.snapshots.value.filter(value => value.connectionId === props.connectionId));
const states = { created: '等待传输', transferring: '传输中', finalizing: '正在完成', completed: '已完成', cancelled: '已取消', failed: '失败' };
</script>
<template>
  <aside class="transfer-panel" aria-label="传输任务">
    <header><h2>传输任务</h2><BaseButton :disabled="!tasks.some(task => task.state === 'completed')" @click="store.clearCompleted(connectionId)">清除已完成</BaseButton></header>
    <p v-if="store.errors.value[connectionId]" role="alert">{{ presentError(store.errors.value[connectionId]!).message }}</p>
    <p v-if="!tasks.length" class="files-empty">暂无传输任务。文件通过 Core 直接传输。</p>
    <ol><li v-for="task in tasks" :key="task.transferId" :aria-label="`${task.fileName} ${states[task.state]}`">
      <div class="transfer-title"><strong :title="task.finalPath">{{ task.fileName }}</strong><span>{{ task.direction === 'upload' ? '上传' : '下载' }}</span></div>
      <progress :max="100" :value="task.state === 'completed' ? 100 : progress(task.totalBytes, task.transferredBytes) ?? (transferFinished(task) ? 0 : undefined)" :aria-label="`${task.fileName} 进度`" />
      <div class="transfer-details"><span :title="`${task.transferredBytes} / ${task.totalBytes ?? '未知'} bytes`">{{ formatSize(task.transferredBytes) }} / {{ formatSize(task.totalBytes) }}</span><span>{{ states[task.state] }}</span></div>
      <p v-if="!transferFinished(task) && task.bytesPerSecond !== null">{{ formatSize(String(Math.round(task.bytesPerSecond))) }}/s<span v-if="task.remainingSeconds !== null"> · 预计剩余 {{ String(task.remainingSeconds) }} 秒</span></p>
      <p v-if="task.error || store.errors.value[task.transferId]" role="alert">{{ presentError((task.error ?? store.errors.value[task.transferId])!).message }}</p>
      <p v-if="task.cleanupRequired" role="alert">临时文件清理未完成，请核实后处理：<code>{{ task.temporaryPath ?? '路径未知' }}</code></p>
      <BaseButton v-if="!transferFinished(task)" :disabled="store.cancelling.value.includes(task.transferId)" @click="store.cancel(task.transferId)">{{ store.cancelling.value.includes(task.transferId) ? '正在取消…' : '取消传输' }}</BaseButton>
    </li></ol>
  </aside>
</template>
